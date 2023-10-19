use crate::{caddr_alloc, CADDR_DEVMEM, CADDR_IRQ_CONTROL, CADDR_MEM, CADDR_VSPACE};
use bitflags::{bitflags, Flags};
use core::mem;
use librust::syscall_abi::identify::CapabilityVariant;
use librust::{prelude::CAddr, syscall_abi::MapFlags};
use librust::{print, println};
use regs::{RO, RW, WO};

const VIRTIO_DEVICE: usize = 0x10008000;
const VIRTIO_DEVICE_LEN: usize = 0x1000;

const VIRTIO_MAGIC: u32 = 0x74726976;

macro_rules! back_to_enum {
    ($(#[$meta:meta])* $vis:vis enum $name:ident {
        $($(#[$vmeta:meta])* $vname:ident $(= $val:expr)?,)*
    }) => {
        $(#[$meta])*
        $vis enum $name {
            $($(#[$vmeta])* $vname $(= $val)?,)*
        }

        impl core::convert::TryFrom<u8> for $name {
            type Error = ();

            fn try_from(v: u8) -> Result<Self, Self::Error> {
                match v {
                    $(x if x == $name::$vname as u8 => Ok($name::$vname),)*
                    _ => Err(()),
                }
            }
        }
    }
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
#[repr(u32)]
pub enum DeviceId {
    INVALID = 0,
    NETWORK_CARD = 1,
    BLOCK_DEVICE = 2,
    CONSOLE = 3,
    ENTROPY_SOURCE = 4,
    MEMORY_BALLOONING_TRADITIONAL = 5,
    IO_MEMORY = 6,
    RPMSG = 7,
    SCSI_HOST = 8,
    NINEP_TRANSPORT = 9,
    MAC80211_WLAN = 10,
    RPROC_SERIAL = 11,
    VIRTIO_CAIF = 12,
    MEMORY_BALLOON = 13,
    GPU_DEVICE = 16,
    TIMER_CLOCK_DEVICE = 17,
    INPUT_DEVICE = 18,
    SOCKET_DEVICE = 19,
    CRYPTO_DEVICE = 20,
    SIGNAL_DISTRIBUTION_MODULE = 21,
    PSTORE_DEVICE = 22,
    IOMMU_DEVICE = 23,
    MEMORY_DEVICE = 24,
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
#[repr(u32)]
enum DeviceStatus {
    /// Indicates that the guest OS has found the device and recognized it as a valid virtio device.
    ACKNOWLEDGE = 1,
    /// Dictates that the guest OS knows how to drive the device.
    ///
    /// **Note:** There could be a significant (or infinite) delay before setting this bit. For example, under Linux, drivers can be loadable modules.
    DRIVER = 2,
    /// Indicates that something went wrong in the guest, and it has given up on the device.
    ///
    /// This could be an internal error, or the driver didn’t like the device for some reason, or even a fatal error during device operation.
    FAILED = 128,
    /// Indicates that the driver has acknowledged all the features it understands, and feature negotiation is complete.
    FEATURES_OK = 8,
    /// Indicates that the driver is set up and ready to drive the device.
    DRIVER_OK = 4,
    /// Indicates that the device has experienced an error from which it can’t recover.
    DEVICE_NEEDS_RESET = 64,
}

bitflags! {
    #[derive(Debug)]
    pub struct DeviceFeaturesLow: u32 {
        const NINEP_TAGGED = 0b1;
        const VIRTIO_F_RING_INDIRECT_DESC = 0b1 << 28;
        const VIRTIO_F_RING_EVENT_IDX = 0b1 << 29;
    }

    #[derive(Debug)]
    pub struct DeviceFeaturesHigh: u32 {
        const VIRTIO_F_VERSION_1 = 0b1 << (32 - 32);
        const VIRTIO_F_ACCESS_PLATFORM = 0b1 << (33- 32);
        const VIRTIO_F_RING_PACKED = 0b1 << (34 - 32);
        const VIRTIO_F_IN_ORDER = 0b1 << (35 - 32);
        const VIRTIO_F_ORDER_PLATFORM = 0b1 << (36 - 32);
        const VIRTIO_F_SR_IOV = 0b1 << (37 - 32);
        const VIRTIO_F_NOTIFICATION_DATA = 0b1 << (38 - 32);
    }
}

/// The registers of a VirtIO device
///
/// See https://docs.oasis-open.org/virtio/virtio/v1.1/csprd01/virtio-v1.1-csprd01.html#x1-1440002
#[repr(C)]
pub struct VirtDevice {
    magic: RO<u32>,
    version: RO<u32>,
    device_id: RO<DeviceId>,
    vendor_id: RO<u32>,
    host_features: RO<u32>,
    host_feauture_sel: WO<u32>,
    _reserved0: [RO<u32>; 2],
    guest_feautures: WO<u32>,
    guest_feauture_sel: WO<u32>,
    pub guest_page_size: WO<u32>,
    _reserved1: RO<u32>,
    queue_sel: WO<u32>,
    queue_num_max: RO<u32>,
    queue_num: WO<u32>,
    queue_align: WO<u32>,
    queue_pfn: RW<u32>,
    _reserved2: [RO<u32>; 3],
    pub queue_notify: WO<u32>,
    _reserved3: [RO<u32>; 3],
    interrupt_status: RO<u32>,
    interrupt_ack: WO<u32>,
    _reserved4: [RO<u32>; 2],
    status: RW<u32>,
}

impl VirtDevice {
    pub fn notify(&self, queue_num: usize) {
        unsafe { self.queue_notify.write(queue_num as u32) }
    }
}

#[repr(u16)]
pub enum DescriptorFlags {
    NEXT = 1,
    WRITE = 2,
    INDIRECT = 4,
}

/// A singular entry in the descriptor table of a VirtIO queue
#[derive(Default, Clone, Copy, Debug)]
#[repr(C)]
pub struct Descriptor {
    pub address: u64,
    pub length: u32,
    pub flags: u16,
    pub next: u16,
}

impl Descriptor {
    pub fn is_free(&self) -> bool {
        self.length == 0 && self.address == 0
    }

    pub fn describe_response(&mut self, resp_buf: &VirtQMsgBuf) {
        self.address = resp_buf.paddr as u64;
        self.length = 4096;
        self.flags = DescriptorFlags::WRITE as u16;
    }

    pub fn describe_request(&mut self, req_buf: &VirtQMsgBuf, resp_idx: usize) {
        self.address = req_buf.paddr as u64;
        self.length = 4096;
        self.next = resp_idx as u16;
        self.flags = DescriptorFlags::NEXT as u16;
    }
}

/// A handle to a VirtIO *available* buffer
#[derive(Debug)]
pub struct VirtQAvail {
    pub flags: &'static mut u16,
    pub idx: &'static mut u16,
    pub ring: &'static mut [u16],
    pub used_events: &'static mut u16,
}

impl VirtQAvail {
    pub fn insert_request(&mut self, desc_idx: u16) {
        self.ring[(*self.idx as usize) % self.ring.len()] = desc_idx;
        *self.idx = self.idx.wrapping_add(1);
    }
}

/// A handle to a VirtIO *used* buffer
#[derive(Debug)]
pub struct VirtQUsed {
    pub flags: &'static mut u16,
    pub idx: &'static mut u16,
    pub ring: &'static mut [VirtQUsedElem],
    pub avail_event: &'static mut u16,
}

#[derive(Debug)]
#[repr(C)]
pub struct VirtQUsedElem {
    pub id: u32,
    pub len: u32,
}

/// A high-level handle to a VirtIO queue.
#[derive(Debug)]
pub struct VirtQ {
    pub descriptor_table: &'static mut [Descriptor],
    pub avail: VirtQAvail,
    pub used: VirtQUsed,
}

impl VirtQ {
    pub fn get_free_descriptor(&mut self) -> Option<(usize, &mut Descriptor)> {
        self.descriptor_table
            .iter_mut()
            .enumerate()
            .find(|(_, d)| d.is_free())
    }
}

fn queue_alloc(queue_bytes: usize) -> Result<(*mut u8, usize), ()> {
    const PAGESIZE: usize = 4096;
    assert_eq!(queue_bytes & (PAGESIZE - 1), 0);
    let pages = queue_bytes / PAGESIZE;

    // choose an arbitrary address to store the queue in...
    // Because this is hardcoded, we can only alloc one queue

    let addr = 0x32_0000_0000 as *mut u8;
    // map one page as buffer because virtqueue pages have to be physically contigious
    // and we can't guarantee that, because mapping in a vspace uses pages..
    {
        let page = caddr_alloc::alloc_caddr();
        librust::derive(CADDR_MEM, page, CapabilityVariant::Page, None).unwrap();
        librust::map_page(
            page,
            CADDR_VSPACE,
            CADDR_MEM,
            addr as usize,
            MapFlags::READ | MapFlags::WRITE,
        )
        .unwrap();
    }
    let addr = (addr as usize + PAGESIZE) as *mut u8;
    let mut paddr = None;
    for i in 0..pages {
        let page = caddr_alloc::alloc_caddr();
        librust::derive(CADDR_MEM, page, CapabilityVariant::Page, None).unwrap();
        let this_paddr = librust::page_paddr(page).unwrap();
        paddr.get_or_insert(this_paddr);
        assert_eq!(
            paddr,
            Some(this_paddr - i * PAGESIZE),
            "non consecutive physical pages for virtio driver"
        );
        librust::map_page(
            page,
            CADDR_VSPACE,
            CADDR_MEM,
            addr as usize + i * PAGESIZE,
            MapFlags::READ | MapFlags::WRITE,
        )
        .unwrap();
    }
    return Ok((addr, paddr.unwrap()));
}

pub fn queue_setup(dev: &mut VirtDevice, queue_num: u32) -> Result<VirtQ, ()> {
    unsafe {
        dev.queue_sel.write(queue_num);
        assert_eq!(dev.queue_pfn.read(), 0);
    }

    let max_items = unsafe {
        let max_items = dev.queue_num_max.read();
        if max_items == 0 {
            return Err(());
        }
        max_items
    };

    let queue_len = core::cmp::min(max_items as usize, 256);

    const PAGESIZE: usize = 4096;
    unsafe {
        dev.queue_num.write(queue_len as u32);
        dev.queue_align.write(PAGESIZE as u32);
    }

    let desc_sz = 16 * queue_len;
    let avail_sz = 6 + 2 * queue_len;
    let used_sz = 6 + 8 * queue_len;

    fn align(s: usize) -> usize {
        const PAGESIZE: usize = 4096;
        let pages = (s + (PAGESIZE - 1)) / PAGESIZE;
        return pages * PAGESIZE;
    }
    let queue_bytes = align(desc_sz + avail_sz) + align(used_sz);
    let (queue_buf, paddr) = queue_alloc(queue_bytes)?;

    assert_eq!(paddr % PAGESIZE, 0);
    unsafe {
        dev.queue_pfn.write((paddr / PAGESIZE) as u32);
    };

    // construct a handler to the just configured queue
    let virtq = unsafe {
        let avail_ptr = queue_buf.add(desc_sz);
        let used_ptr = queue_buf.add(align(desc_sz + avail_sz));

        VirtQ {
            descriptor_table: core::slice::from_raw_parts_mut(queue_buf.cast(), queue_len),
            avail: VirtQAvail {
                flags: avail_ptr.cast::<u16>().add(0).as_mut().unwrap(),
                idx: avail_ptr.cast::<u16>().add(1).as_mut().unwrap(),
                ring: core::slice::from_raw_parts_mut(avail_ptr.cast::<u16>().add(2), queue_len),
                used_events: avail_ptr
                    .cast::<u16>()
                    .add(2)
                    .add(queue_len)
                    .as_mut()
                    .unwrap(),
            },
            used: VirtQUsed {
                flags: used_ptr.cast::<u16>().add(0).as_mut().unwrap(),
                idx: used_ptr.cast::<u16>().add(1).as_mut().unwrap(),
                ring: core::slice::from_raw_parts_mut(
                    used_ptr.cast::<u16>().add(2).cast::<VirtQUsedElem>(),
                    queue_len,
                ),
                avail_event: used_ptr
                    .cast::<VirtQUsedElem>()
                    .add(queue_len)
                    .cast::<u16>()
                    .add(2)
                    .as_mut()
                    .unwrap(),
            },
        }
    };
    return Ok(virtq);
}

#[derive(Debug)]
struct VirtQMsgBuf {
    pub buf: &'static mut [u8],
    pub paddr: usize,
    pub page: CAddr,
}

impl VirtQMsgBuf {
    pub fn clear(&mut self) {
        self.buf.fill(0);
    }
}

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

        let mut queue = queue_setup(device, 0).unwrap();
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

        todo!()
    }
}

back_to_enum! {
    #[derive(Debug, Copy, Clone, Eq, PartialEq)]
    #[repr(u8)]
    pub enum P9MsgType {
        TVersion = 100,
        RVersion = 101,
        TAuth = 102,
        RAuth = 103,
        TAttach = 104,
        RAttach = 105,
        TError = 106,
        RError = 107,
        TFlush = 108,
        RFlush = 109,
        TWalk = 110,
        RWalk = 111,
        TOpen = 112,
        ROpen = 113,
        TCreate = 114,
        RCreate = 115,
        TRead = 116,
        RRead = 117,
        TWrite = 118,
        RWrite = 119,
        TClunk = 120,
        RClunk = 121,
        TRemove = 122,
        RRemove = 123,
        TStat = 124,
        RStat = 125,
        TWStat = 126,
        RWStat = 127,
        TMax = 128,
    }
}

back_to_enum! {
    #[derive(Debug, Copy, Clone, Eq, PartialEq)]
    #[repr(u8)]
    pub enum P9QidType {
        File = 0,
        Link = 1,
        Symlink = 1 << 1,
        Tmp = 1 << 2,
        Auth = 1 << 3,
        Mount = 1 << 4,
        Excl = 1 << 5,
        Append = 1 << 6,
        Dir = 1 << 7,
    }
}

#[derive(Debug)]
pub struct P9Qid {
    pub typ: P9QidType,
    pub version: u32,
    pub path: u64,
}

#[derive(Debug)]
struct P9RequestBuilder<'buf> {
    buf: &'buf mut [u8],
    fill_marker: usize,
}

impl<'buf> P9RequestBuilder<'buf> {
    fn new(buf: &'buf mut [u8]) -> Self {
        Self {
            buf,
            fill_marker: 4,
        }
    }

    fn write_type(&mut self, typ: P9MsgType) -> &mut Self {
        self.write_u8(typ as u8)
    }

    fn write_u8(&mut self, value: u8) -> &mut Self {
        self.buf[self.fill_marker] = value;
        self.fill_marker += 1;
        self
    }

    fn write_u16(&mut self, value: u16) -> &mut Self {
        self.buf[self.fill_marker..self.fill_marker + mem::size_of::<u16>()]
            .copy_from_slice(&value.to_le_bytes());
        self.fill_marker += mem::size_of::<u16>();
        self
    }

    fn write_u32(&mut self, value: u32) -> &mut Self {
        self.buf[self.fill_marker..self.fill_marker + mem::size_of::<u32>()]
            .copy_from_slice(&value.to_le_bytes());
        self.fill_marker += mem::size_of::<u32>();
        self
    }

    fn write_u64(&mut self, value: u64) -> &mut Self {
        self.buf[self.fill_marker..self.fill_marker + mem::size_of::<u64>()]
            .copy_from_slice(&value.to_le_bytes());
        self.fill_marker += mem::size_of::<u64>();
        self
    }

    fn write_str(&mut self, value: &str) -> &mut Self {
        self.write_u16(value.len() as u16);
        self.buf[self.fill_marker..self.fill_marker + value.len()]
            .copy_from_slice(value.as_bytes());
        self.fill_marker += value.len();
        self
    }

    fn finish(&mut self) {
        self.write_u32(self.fill_marker as u32 - 4);
    }
}

#[derive(Debug)]
pub struct P9ResponseReader<'buf> {
    buf: &'buf [u8],
    pub msg_type: P9MsgType,
    pos: u32,
}

impl<'buf> P9ResponseReader<'buf> {
    pub fn new(buf: &'buf [u8]) -> Self {
        let msg_length = u32::from_le_bytes((&buf[0..4]).try_into().unwrap());
        let msg_type = P9MsgType::try_from(buf[4]).unwrap();

        Self {
            buf: &buf[0..msg_length as usize],
            msg_type,
            pos: 5,
        }
    }

    pub fn read_u8(&mut self) -> Option<u8> {
        self.buf.get(self.pos as usize).map(|&result| {
            self.pos += 1;
            result
        })
    }

    pub fn read_u16(&mut self) -> Option<u16> {
        self.buf
            .get(self.pos as usize..self.pos as usize + mem::size_of::<u16>())
            .map(|result| {
                self.pos += mem::size_of::<u16>() as u32;
                u16::from_le_bytes(result.try_into().unwrap())
            })
    }

    pub fn read_u32(&mut self) -> Option<u32> {
        self.buf
            .get(self.pos as usize..self.pos as usize + mem::size_of::<u32>())
            .map(|result| {
                self.pos += mem::size_of::<u32>() as u32;
                u32::from_le_bytes(result.try_into().unwrap())
            })
    }

    pub fn read_u64(&mut self) -> Option<u64> {
        self.buf
            .get(self.pos as usize..self.pos as usize + mem::size_of::<u64>())
            .map(|result| {
                self.pos += mem::size_of::<u64>() as u32;
                u64::from_le_bytes(result.try_into().unwrap())
            })
    }

    pub fn read_str(&mut self) -> Option<&str> {
        let str_len = self.read_u16()?;
        self.buf
            .get(self.pos as usize..self.pos as usize + str_len as usize)
            .map(|str_bytes| {
                self.pos += str_len as u32;
                core::str::from_utf8(str_bytes).unwrap()
            })
    }

    pub fn read_qid(&mut self) -> Option<P9Qid> {
        let typ = P9QidType::try_from(self.read_u8()?).unwrap();
        let version = self.read_u32()?;
        let path = self.read_u64()?;
        Some(P9Qid { typ, version, path })
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

    P9RequestBuilder::new(req_buf.buf)
        .write_type(P9MsgType::TVersion)
        .write_u16(!0)
        .write_u32(4096)
        .write_str("9P2000.u")
        .finish();

    exchange_p9_virtio_msgs(device, queue, irq_notif, req_buf, resp_buf);

    let mut resp = P9ResponseReader::new(resp_buf.buf);
    assert_eq!(resp.msg_type, P9MsgType::RVersion);
    let tag = resp.read_u16().unwrap();
    assert_eq!(tag, !0);
    let msize = resp.read_u32().unwrap();
    assert_eq!(msize, 4096);
    let version = resp.read_str().unwrap();
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

    P9RequestBuilder::new(req_buf.buf)
        .write_type(P9MsgType::TAttach)
        .write_u16(!0)
        .write_u32(fid)
        .write_u32(!0)
        .write_str(uname)
        .write_str(aname)
        .finish();

    exchange_p9_virtio_msgs(device, queue, irq_notif, req_buf, resp_buf);

    let mut resp = P9ResponseReader::new(resp_buf.buf);
    assert_eq!(resp.msg_type, P9MsgType::RAttach);
    let _tag = resp.read_u16().unwrap();
    let qid = resp.read_qid().unwrap();

    librust::irq_complete(irq).unwrap();
    qid
}
