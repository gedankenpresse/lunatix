use core::panic;

use crate::drivers::p9::ByteReader;
use crate::drivers::virtio::{
    self, DeviceFeaturesLow, DeviceId, DeviceStatus, VirtDevice, VIRTIO_MAGIC,
};
use crate::{caddr_alloc, CADDR_DEVMEM, CADDR_IRQ_CONTROL, CADDR_MEM, CADDR_VSPACE};

use librust::println;
use librust::syscall_abi::identify::CapabilityVariant;
use librust::{prelude::CAddr, syscall_abi::MapFlags};

use super::p9::{
    self, P9FileFlags, P9FileMode, P9Qid, P9RequestBuilder, ROpen, RRead, RVersion, RWalk,
    Response, TAttach, TOpen, TRead, TVersion, TWalk,
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
    unsafe {
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
        return P9Driver {
            device,
            queue,
            noti: irq_notif,
            irq,
            req: req_buf,
            res: resp_buf,
        };
    }
}

pub fn test() {
    let mut driver = init_9p_driver();
    p9_handshake(&mut driver);

    let attach_fid = 1;
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

    let walk_fid = attach_fid;
    /*
    let _ = p9_walk(
        &mut driver,
        TWalk {
            tag: 1,
            fid: attach_fid,
            newfid: walk_fid,
            wnames: &[],
        },
    );
    */
    let _ = p9_open(
        &mut driver,
        TOpen {
            tag: 2,
            fid: walk_fid,
            mode: P9FileMode::OREAD,
            flags: P9FileFlags::empty(),
        },
    );

    let root_info = p9_read(
        &mut driver,
        TRead {
            tag: !0,
            fid: walk_fid,
            offset: 0,
            count: 512,
        },
    );
    let mut dir_entry_reader = ByteReader::new(root_info.data);

    let stat = p9::Stat::deserialize(&mut dir_entry_reader).unwrap();
    println!("{:#?}", stat);

    todo!()
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

fn p9_read<'resp>(driver: &'resp mut P9Driver, read: TRead) -> RRead<'resp> {
    let irq = driver.irq;
    let res = driver.do_request(p9::Request::Read(read)).unwrap();
    let Response::Read(resp) = res else { panic!() };

    librust::irq_complete(irq).unwrap();
    resp
}
