use crate::{caddr_alloc, CADDR_MEM, CADDR_VSPACE};
use bitflags::{bitflags, Flags};
use librust::syscall_abi::{identify::CapabilityVariant, CAddr, MapFlags};
use regs::{RO, RW, WO};

pub const VIRTIO_MAGIC: u32 = 0x74726976;

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
pub enum DeviceStatus {
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
    pub magic: RO<u32>,
    pub version: RO<u32>,
    pub device_id: RO<DeviceId>,
    pub vendor_id: RO<u32>,
    pub host_features: RO<u32>,
    pub host_feauture_sel: WO<u32>,
    _reserved0: [RO<u32>; 2],
    pub guest_feautures: WO<u32>,
    pub guest_feauture_sel: WO<u32>,
    guest_page_size: WO<u32>,
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
    pub status: RW<u32>,
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
pub struct VirtQMsgBuf {
    pub buf: &'static mut [u8],
    pub paddr: usize,
    pub page: CAddr,
}

impl VirtQMsgBuf {
    pub fn clear(&mut self) {
        self.buf.fill(0);
    }
}
