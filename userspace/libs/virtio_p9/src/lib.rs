#![no_std]

use core::alloc::Layout;
use core::panic;

use io::read::Reader;
use virtio::{DeviceFeaturesLow, DeviceId, VirtDeviceMM};

use caddr_alloc;
use liblunatix::prelude::syscall_abi::MapFlags;
use liblunatix::prelude::CAddr;
use liblunatix::{prelude::syscall_abi::identify::CapabilityVariant, MemoryPage};

use p9::{
    P9FileFlags, P9FileMode, P9Qid, P9RequestBuilder, RClunk, ROpen, RRead, RVersion, RWalk,
    Response, TAttach, TClunk, TOpen, TRead, TVersion, TWalk,
};
use virtio::{VirtQ, VirtQMsgBuf};

const VIRTIO_DEVICE: usize = 0x10008000;
const VIRTIO_DEVICE_LEN: usize = 0x1000;

/// Allocate two buffers from the memory capability that are used for storing the actual P9 messages
fn prepare_msg_bufs(mem: CAddr, vspace: CAddr) -> (VirtQMsgBuf, VirtQMsgBuf) {
    let buf_region = mmap::allocate_raw(Layout::new::<MemoryPage>()).unwrap();
    let page1 = caddr_alloc::alloc_caddr();
    liblunatix::ipc::mem::derive(mem, page1, CapabilityVariant::Page, None).unwrap();
    liblunatix::ipc::page::map_page(
        page1,
        vspace,
        mem,
        buf_region.start as usize,
        MapFlags::READ | MapFlags::WRITE,
    )
    .unwrap();

    let buf2_region = mmap::allocate_raw(Layout::new::<MemoryPage>()).unwrap();
    let page2 = caddr_alloc::alloc_caddr();
    liblunatix::ipc::mem::derive(mem, page2, CapabilityVariant::Page, None).unwrap();
    liblunatix::ipc::page::map_page(
        page2,
        vspace,
        mem,
        buf2_region.start as usize,
        MapFlags::READ | MapFlags::WRITE,
    )
    .unwrap();

    (
        VirtQMsgBuf {
            buf: unsafe { core::slice::from_raw_parts_mut(buf_region.start, buf_region.bytes) },
            page: page1,
            paddr: liblunatix::ipc::page::get_paddr(page1).unwrap(),
        },
        VirtQMsgBuf {
            buf: unsafe { core::slice::from_raw_parts_mut(buf2_region.start, buf2_region.bytes) },
            page: page2,
            paddr: liblunatix::ipc::page::get_paddr(page2).unwrap(),
        },
    )
}

pub fn init_9p_driver(
    mem: CAddr,
    vspace: CAddr,
    devmem: CAddr,
    irq_control: CAddr,
) -> P9Driver<'static> {
    liblunatix::ipc::devmem::devmem_map(devmem, mem, vspace, VIRTIO_DEVICE, VIRTIO_DEVICE_LEN)
        .unwrap();
    let mut driver = unsafe {
        let device = VirtDeviceMM::at(VIRTIO_DEVICE as *mut VirtDeviceMM);
        assert_eq!(device.device_id.read(), DeviceId::NINEP_TRANSPORT);

        // init device according to the docs
        // see https://docs.oasis-open.org/virtio/virtio/v1.1/csprd01/virtio-v1.1-csprd01.html#x1-920001

        let mut status = device.init();
        status = device.negotiate_features(status, DeviceFeaturesLow::NINEP_TAGGED.bits() as u64);

        // setup an irq handler for the virtio device
        let irq_notif = caddr_alloc::alloc_caddr();
        liblunatix::ipc::mem::derive(mem, irq_notif, CapabilityVariant::Notification, None)
            .unwrap();
        let irq = caddr_alloc::alloc_caddr();
        liblunatix::ipc::irq_control::irq_control_claim(irq_control, 0x08, irq, irq_notif).unwrap();

        let queue = virtio::queue_setup(device, 0, mem, vspace).unwrap();
        let (req_buf, resp_buf) = prepare_msg_bufs(mem, vspace);

        device.finish_setup(status);
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
    device: &'mm VirtDeviceMM,
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
        let req_idx = {
            let (req_idx, req_descriptor) = self.queue.get_free_descriptor().unwrap();
            req_descriptor.describe_request(&self.req, resp_idx);
            self.queue.avail.insert_request(req_idx as u16);
            req_idx
        };

        self.device.notify(0);
        liblunatix::syscalls::wait_on(self.noti).unwrap();
        self.queue.descriptor_table[resp_idx].free();
        self.queue.descriptor_table[req_idx].free();
    }

    pub fn read_file<'a>(&'a mut self, path: &[&str]) -> Result<FileReader<'a, 'mm>, &'a str> {
        let fid = 123;
        let _ = p9_walk(
            self,
            TWalk {
                tag: 1,
                fid: 0,
                newfid: fid,
                wnames: path,
            },
        );
        let _open_res = p9_open(
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

    // TODO: fix dir reader
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
        let _open_res = p9_open(
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
    let Response::Version(RVersion {
        tag,
        msize,
        version,
    }) = res
    else {
        panic!()
    };

    assert_eq!(tag, !0);
    assert_eq!(msize, 4096);
    assert_eq!(version, "9P2000.u");

    liblunatix::ipc::irq::irq_complete(irq).unwrap();
}

/// Attach us to a servers file tree
///
/// - uname describes the user
/// - aname describes the file tree to access
/// - fid is the file descriptor id to which the file tree is attached
fn p9_attach(driver: &mut P9Driver, attach: TAttach) -> P9Qid {
    let res = driver.do_request(p9::Request::Attach(attach)).unwrap();
    let Response::Attach(resp) = res else {
        panic!()
    };

    liblunatix::ipc::irq::irq_complete(driver.irq).unwrap();
    resp.qid
}

/// Walk the directory tree to a new directory (effectively chdir)
fn p9_walk(driver: &mut P9Driver, walk: TWalk) -> RWalk {
    let res = driver.do_request(p9::Request::Walk(walk)).unwrap();
    let Response::Walk(resp) = res else { panic!() };
    liblunatix::ipc::irq::irq_complete(driver.irq).unwrap();
    resp
}

fn p9_open(driver: &mut P9Driver, open: TOpen) -> ROpen {
    let res = driver.do_request(p9::Request::Open(open)).unwrap();
    let Response::Open(resp) = res else { panic!() };

    liblunatix::ipc::irq::irq_complete(driver.irq).unwrap();
    resp
}

fn p9_clunk(driver: &mut P9Driver, clunk: TClunk) -> RClunk {
    let res = driver.do_request(p9::Request::Clunk(clunk)).unwrap();
    let Response::Clunk(resp) = res else { panic!() };

    liblunatix::ipc::irq::irq_complete(driver.irq).unwrap();
    resp
}

fn p9_read<'resp>(driver: &'resp mut P9Driver, read: TRead) -> RRead<'resp> {
    let irq = driver.irq;
    let res = driver.do_request(p9::Request::Read(read)).unwrap();
    let Response::Read(resp) = res else { panic!() };

    liblunatix::ipc::irq::irq_complete(irq).unwrap();
    resp
}
