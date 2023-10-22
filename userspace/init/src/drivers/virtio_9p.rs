use core::panic;

use crate::drivers::p9::ByteReader;
use crate::drivers::virtio::{
    self, DeviceFeaturesLow, DeviceId, DeviceStatus, VirtDevice, VIRTIO_MAGIC,
};
use crate::read::Reader;
use crate::{caddr_alloc, CADDR_DEVMEM, CADDR_IRQ_CONTROL, CADDR_MEM, CADDR_VSPACE};

use librust::syscall_abi::identify::CapabilityVariant;
use librust::{prelude::CAddr, syscall_abi::MapFlags};

use super::p9::{
    self, P9FileFlags, P9FileMode, P9Qid, P9RequestBuilder, RClunk, ROpen, RRead, RVersion, RWalk,
    Response, Stat, TAttach, TClunk, TOpen, TRead, TVersion, TWalk,
};
use super::virtio::{VirtQ, VirtQMsgBuf};

const VIRTIO_DEVICE: usize = 0x10008000;
const VIRTIO_DEVICE_LEN: usize = 0x1000;

/// Allocate two buffers from the memory capability that are used for storing the actual P9 messages
fn prepare_msg_bufs() -> (VirtQMsgBuf, VirtQMsgBuf) {
    const BUF1: *mut u8 = 0x30_0000_0000usize as *mut u8;
    let page1 = caddr_alloc::alloc_caddr();
    librust::derive(CADDR_MEM, page1, CapabilityVariant::Page, None).unwrap();
    librust::map_page(
        page1,
        CADDR_VSPACE,
        CADDR_MEM,
        BUF1 as usize,
        MapFlags::READ | MapFlags::WRITE,
    )
    .unwrap();

    const BUF2: *mut u8 = (0x30_0000_0000usize + 4096usize) as *mut u8;
    let page2 = caddr_alloc::alloc_caddr();
    librust::derive(CADDR_MEM, page2, CapabilityVariant::Page, None).unwrap();
    librust::map_page(
        page2,
        CADDR_VSPACE,
        CADDR_MEM,
        BUF2 as usize,
        MapFlags::READ | MapFlags::WRITE,
    )
    .unwrap();

    (
        VirtQMsgBuf {
            buf: unsafe { core::slice::from_raw_parts_mut(BUF1, 4096) },
            page: page1,
            paddr: librust::page_paddr(page1).unwrap(),
        },
        VirtQMsgBuf {
            buf: unsafe { core::slice::from_raw_parts_mut(BUF2, 4096) },
            page: page2,
            paddr: librust::page_paddr(page2).unwrap(),
        },
    )
}

pub fn init_9p_driver() -> P9Driver<'static> {
    librust::devmem_map(
        CADDR_DEVMEM,
        CADDR_MEM,
        CADDR_VSPACE,
        VIRTIO_DEVICE,
        VIRTIO_DEVICE_LEN,
    )
    .unwrap();
    let mut driver = unsafe {
        let device = &mut *(VIRTIO_DEVICE as *mut VirtDevice);
        assert_eq!(device.magic.read(), VIRTIO_MAGIC);
        assert_eq!(device.version.read(), 0x1);
        assert_eq!(device.device_id.read(), DeviceId::NINEP_TRANSPORT);

        // init device according to the docs
        // see https://docs.oasis-open.org/virtio/virtio/v1.1/csprd01/virtio-v1.1-csprd01.html#x1-920001

        // properly acknowledge the device presence
        device.status.write(0x0);
        let mut device_status = device.status.read();
        device_status |= DeviceStatus::ACKNOWLEDGE as u32;
        device.status.write(device_status);
        device_status |= DeviceStatus::DRIVER as u32;
        device.status.write(device_status);

        // negotiate features
        device.host_feauture_sel.write(0);
        let features_low = DeviceFeaturesLow::from_bits_retain(device.host_features.read());
        assert!(features_low.intersects(DeviceFeaturesLow::NINEP_TAGGED));
        device.guest_feauture_sel.write(0);
        device
            .guest_feautures
            .write(DeviceFeaturesLow::NINEP_TAGGED.bits());
        device_status |= DeviceStatus::FEATURES_OK as u32;
        device.status.write(device_status);
        assert_eq!(
            device.status.read() & DeviceStatus::FEATURES_OK as u32,
            DeviceStatus::FEATURES_OK as u32
        );

        // setup an irq handler for the virtio device
        let irq_notif = caddr_alloc::alloc_caddr();
        librust::derive(CADDR_MEM, irq_notif, CapabilityVariant::Notification, None).unwrap();
        let irq = caddr_alloc::alloc_caddr();
        librust::irq_control_claim(CADDR_IRQ_CONTROL, 0x08, irq, irq_notif).unwrap();

        let queue = virtio::queue_setup(device, 0).unwrap();
        let (req_buf, resp_buf) = prepare_msg_bufs();

        // finish device initialization
        device_status |= DeviceStatus::DRIVER_OK as u32;
        device.status.write(device_status);
        P9Driver {
            device,
            queue,
            noti: irq_notif,
            irq,
            req: req_buf,
            res: resp_buf,
        }
    };

    p9_handshake(&mut driver);

    let attach_fid = 0;
    let _ = p9_attach(
        &mut driver,
        TAttach {
            tag: !0,
            fid: attach_fid,
            afid: !0,
            uname: "lunatix",
            aname: "/",
        },
    );
    return driver;
}

pub struct P9Driver<'mm> {
    device: &'mm VirtDevice,
    queue: VirtQ,
    noti: CAddr,
    irq: CAddr,
    req: VirtQMsgBuf,
    res: VirtQMsgBuf,
}

impl<'mm> P9Driver<'mm> {
    pub fn do_request(&mut self, req: p9::Request) -> Result<p9::Response, &'_ str> {
        self.req.clear();
        self.res.clear();

        let req_builder = P9RequestBuilder::new(self.req.buf);
        match req {
            p9::Request::Version(msg) => msg.serialize(req_builder),
            p9::Request::Attach(msg) => msg.serialize(req_builder),
            p9::Request::Walk(msg) => msg.serialize(req_builder),
            p9::Request::Read(msg) => msg.serialize(req_builder),
            p9::Request::Open(msg) => msg.serialize(req_builder),
            p9::Request::Clunk(msg) => msg.serialize(req_builder),
        }

        self.exchange_p9_virtio_msgs();

        let res = Response::deserialize(self.res.buf).unwrap();
        match res {
            Response::Error(e) => Err(e.ename),
            _ => Ok(res),
        }
    }

    /// Send the message in `req_buf` to the VirtIO device described by `device` and `queue` and wait until a response is
    /// sent by the device which should be written into `resp_buf`.
    fn exchange_p9_virtio_msgs(&mut self) {
        let resp_idx = {
            let (resp_idx, resp_descriptor) = self.queue.get_free_descriptor().unwrap();
            resp_descriptor.describe_response(&self.res);
            resp_idx
        };
        {
            let (req_idx, req_descriptor) = self.queue.get_free_descriptor().unwrap();
            req_descriptor.describe_request(&self.req, resp_idx);
            self.queue.avail.insert_request(req_idx as u16);
        }

        self.device.notify(0);
        librust::wait_on(self.noti).unwrap();
    }

    pub fn read_file<'a>(&'a mut self, path: &str) -> Result<FileReader<'a, 'mm>, &'a str> {
        let fid = 123;
        let _ = p9_walk(
            self,
            TWalk {
                tag: 1,
                fid: 0,
                newfid: fid,
                wnames: &[path],
            },
        );
        let open_res = p9_open(
            self,
            TOpen {
                tag: 1,
                fid,
                mode: P9FileMode::OREAD,
                flags: P9FileFlags::empty(),
            },
        );

        Ok(FileReader {
            driver: self,
            fid,
            pos: 0,
        })
    }

    pub(crate) fn read_dir<'a>(&'a mut self) -> Result<DirReader<'a, 'mm>, &'a str> {
        let fid = 1234;
        let _ = p9_walk(
            self,
            TWalk {
                tag: 1,
                fid: 0,
                newfid: fid,
                wnames: &[],
            },
        );
        let open_res = p9_open(
            self,
            TOpen {
                tag: 1,
                fid,
                mode: P9FileMode::OREAD,
                flags: P9FileFlags::empty(),
            },
        );

        Ok(DirReader {
            driver: self,
            fid,
            pos: 0,
        })
    }
}

pub struct DirReader<'a, 'mm> {
    driver: &'a mut P9Driver<'mm>,
    fid: u32,
    pos: u64,
}

impl<'a, 'mm> Drop for DirReader<'a, 'mm> {
    fn drop(&mut self) {
        p9_clunk(
            &mut self.driver,
            TClunk {
                tag: !0,
                fid: self.fid,
            },
        );
    }
}

impl<'a, 'mm> DirReader<'a, 'mm> {
    pub fn read_entry<'b: 's, 's>(&'b mut self) -> Option<Stat<'s>> {
        let res = p9_read(
            self.driver,
            TRead {
                tag: 1,
                fid: self.fid,
                offset: self.pos,
                count: 512,
            },
        );
        if res.data.len() == 0 {
            return None;
        }
        // NOTE: this is a hack to advance exactly one directory entry...
        let size = {
            let mut reader = ByteReader::new(res.data);
            reader.read_u16()?
        };

        let stat = Stat::deserialize(&mut ByteReader::new(res.data))?;
        self.pos += size as u64;
        Some(stat)
    }
}

pub struct FileReader<'a, 'mm> {
    driver: &'a mut P9Driver<'mm>,
    fid: u32,
    pos: u64,
}

impl<'a, 'mm> Drop for FileReader<'a, 'mm> {
    fn drop(&mut self) {
        p9_clunk(
            &mut self.driver,
            TClunk {
                tag: !0,
                fid: self.fid,
            },
        );
    }
}

impl<'a, 'mm> Reader for FileReader<'a, 'mm> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, ()> {
        let len = core::cmp::min(buf.len(), 4000) as u32; // Choose best size here that fits in req buff
        let res = p9_read(
            self.driver,
            TRead {
                tag: 1,
                fid: self.fid,
                offset: self.pos,
                count: len,
            },
        );
        let data = res.data;
        buf[0..data.len()].copy_from_slice(data);
        self.pos += data.len() as u64;
        Ok(data.len())
    }
}

/// Perform a P9 handshake to introduce us to the server and negotiate a version
fn p9_handshake(driver: &mut P9Driver) {
    let msg = TVersion {
        msize: 4096,
        version: "9P2000.u",
    };
    let irq = driver.irq;
    let res = driver.do_request(p9::Request::Version(msg)).unwrap();
    let Response::Version(RVersion { tag, msize, version }) = res else { panic!() };

    assert_eq!(tag, !0);
    assert_eq!(msize, 4096);
    assert_eq!(version, "9P2000.u");

    librust::irq_complete(irq).unwrap();
}

/// Attach us to a servers file tree
///
/// - uname describes the user
/// - aname describes the file tree to access
/// - fid is the file descriptor id to which the file tree is attached
fn p9_attach(driver: &mut P9Driver, attach: TAttach) -> P9Qid {
    let res = driver.do_request(p9::Request::Attach(attach)).unwrap();
    let Response::Attach(resp) = res else { panic!() };

    librust::irq_complete(driver.irq).unwrap();
    resp.qid
}

/// Walk the directory tree to a new directory (effectively chdir)
fn p9_walk(driver: &mut P9Driver, walk: TWalk) -> RWalk {
    let res = driver.do_request(p9::Request::Walk(walk)).unwrap();
    let Response::Walk(resp) = res else { panic!() };
    librust::irq_complete(driver.irq).unwrap();
    resp
}

fn p9_open(driver: &mut P9Driver, open: TOpen) -> ROpen {
    let res = driver.do_request(p9::Request::Open(open)).unwrap();
    let Response::Open(resp) = res else { panic!() };

    librust::irq_complete(driver.irq).unwrap();
    resp
}

fn p9_clunk(driver: &mut P9Driver, clunk: TClunk) -> RClunk {
    let res = driver.do_request(p9::Request::Clunk(clunk)).unwrap();
    let Response::Clunk(resp) = res else { panic!() };

    librust::irq_complete(driver.irq).unwrap();
    resp
}

fn p9_read<'resp>(driver: &'resp mut P9Driver, read: TRead) -> RRead<'resp> {
    let irq = driver.irq;
    let res = driver.do_request(p9::Request::Read(read)).unwrap();
    let Response::Read(resp) = res else { panic!() };

    librust::irq_complete(irq).unwrap();
    resp
}
