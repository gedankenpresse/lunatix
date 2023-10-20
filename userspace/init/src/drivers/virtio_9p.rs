use crate::drivers::p9::{
    P9MsgType, P9RequestBuilder, RVersion, RWalk, Response, TAttach, TVersion, TWalk,
};
use crate::drivers::virtio::{
    self, DeviceFeaturesLow, DeviceId, DeviceStatus, VirtDevice, VIRTIO_MAGIC,
};
use crate::{caddr_alloc, CADDR_DEVMEM, CADDR_IRQ_CONTROL, CADDR_MEM, CADDR_VSPACE};

use librust::syscall_abi::identify::CapabilityVariant;
use librust::{prelude::CAddr, syscall_abi::MapFlags};
use librust::{print, println};

use super::p9::P9Qid;
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

pub fn test() {
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

        let mut queue = virtio::queue_setup(device, 0).unwrap();
        let (mut req_buf, mut resp_buf) = prepare_msg_bufs();

        // finish device initialization
        device_status |= DeviceStatus::DRIVER_OK as u32;
        device.status.write(device_status);

        p9_handshake(
            &device,
            &mut queue,
            irq_notif,
            irq,
            &mut req_buf,
            &mut resp_buf,
        );
        let root_fid = 1;
        let root_qid = p9_attach(
            &device,
            &mut queue,
            irq_notif,
            irq,
            &mut req_buf,
            &mut resp_buf,
            "lunatix",
            "/",
            root_fid,
        );

        println!("root_qid={root_qid:?}");
        let walk_fid = 2;
        let walk_result = p9_walk(
            &device,
            &mut queue,
            irq_notif,
            irq,
            &mut req_buf,
            &mut resp_buf,
            root_fid,
            walk_fid,
            &[".", "index.txt"],
        );
        println!("{:#?}", walk_result.qids());

        todo!()
    }
}

/// Send the message in `req_buf` to the VirtIO device described by `device` and `queue` and wait until a response is
/// sent by the device which should be written into `resp_buf`.
fn exchange_p9_virtio_msgs(
    device: &VirtDevice,
    queue: &mut VirtQ,
    irq_notif: CAddr,
    req_buf: &VirtQMsgBuf,
    resp_buf: &VirtQMsgBuf,
) {
    let resp_idx = {
        let (resp_idx, resp_descriptor) = queue.get_free_descriptor().unwrap();
        resp_descriptor.describe_response(resp_buf);
        resp_idx
    };
    {
        let (req_idx, req_descriptor) = queue.get_free_descriptor().unwrap();
        req_descriptor.describe_request(req_buf, resp_idx);
        queue.avail.insert_request(req_idx as u16);
    }

    device.notify(0);
    print!("waiting for virtio response...");
    librust::wait_on(irq_notif).unwrap();
    println!("...done")
}

/// Perform a P9 handshake to introduce us to the server and negotiate a version
fn p9_handshake(
    device: &VirtDevice,
    queue: &mut VirtQ,
    irq_notif: CAddr,
    irq: CAddr,
    req_buf: &mut VirtQMsgBuf,
    resp_buf: &mut VirtQMsgBuf,
) {
    req_buf.clear();
    resp_buf.clear();

    let msg = TVersion {
        msize: 4096,
        version: "9P2000.u",
    };
    msg.serialize(P9RequestBuilder::new(req_buf.buf));

    exchange_p9_virtio_msgs(device, queue, irq_notif, req_buf, resp_buf);

    let resp = Response::deserialize(resp_buf.buf).unwrap();
    let Response::Version(RVersion {
        tag,
        msize,
        version,
    }) = resp
    else {
        panic!()
    };

    assert_eq!(tag, !0);
    assert_eq!(msize, 4096);
    assert_eq!(version, "9P2000.u");

    librust::irq_complete(irq).unwrap();
    println!("got 9p handshake response: msize={msize} version={version}");
}

/// Attach us to a servers file tree
///
/// - uname describes the user
/// - aname describes the file tree to access
/// - fid is the file descriptor id to which the file tree is attached
fn p9_attach(
    device: &VirtDevice,
    queue: &mut VirtQ,
    irq_notif: CAddr,
    irq: CAddr,
    req_buf: &mut VirtQMsgBuf,
    resp_buf: &mut VirtQMsgBuf,
    uname: &str,
    aname: &str,
    fid: u32,
) -> P9Qid {
    req_buf.clear();
    resp_buf.clear();

    let msg = TAttach {
        tag: !0,
        fid,
        afid: !0,
        uname,
        aname,
    };
    msg.serialize(P9RequestBuilder::new(req_buf.buf));

    exchange_p9_virtio_msgs(device, queue, irq_notif, req_buf, resp_buf);

    let resp = Response::deserialize(resp_buf.buf).unwrap();
    let Response::Attach(resp) = resp else {
        panic!()
    };

    librust::irq_complete(irq).unwrap();
    resp.qid
}

/// Walk the
fn p9_walk(
    device: &VirtDevice,
    queue: &mut VirtQ,
    irq_notif: CAddr,
    irq: CAddr,
    req_buf: &mut VirtQMsgBuf,
    resp_buf: &mut VirtQMsgBuf,
    fid: u32,
    newfid: u32,
    wnames: &[&str],
) -> RWalk {
    req_buf.clear();
    resp_buf.clear();

    let msg = TWalk {
        tag: !0,
        fid,
        newfid,
        wnames,
    };
    msg.serialize(P9RequestBuilder::new(req_buf.buf));

    exchange_p9_virtio_msgs(device, queue, irq_notif, req_buf, resp_buf);

    let Response::Walk(resp) = Response::deserialize(resp_buf.buf).unwrap() else {
        panic!()
    };
    librust::irq_complete(irq).unwrap();
    resp
}
