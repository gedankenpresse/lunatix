use crate::{caddr_alloc, CADDR_DEVMEM, CADDR_MEM, CADDR_VSPACE};
use bitflags::{bitflags, Flags};
use librust::println;
use librust::syscall_abi::identify::CapabilityVariant;
use librust::{prelude::CAddr, syscall_abi::MapFlags};
use regs::{RO, RW, WO};

const VIRTIO_DEVICE: usize = 0x10008000;
const VIRTIO_DEVICE_LEN: usize = 0x1000;

const VIRTIO_MAGIC: u32 = 0x74726976;

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

#[repr(u16)]
pub enum DescriptorFlags {
    NEXT = 1,
    WRITE = 2,
    INDIRECT = 4,
}

#[derive(Default, Clone, Copy, Debug)]
#[repr(C)]
pub struct Descriptor {
    pub address: u64,
    pub length: u32,
    pub flags: u16,
    pub next: u16,
}

pub struct QueueBuf {
    size: u16,
    buf: *mut u8,
}

fn queue_alloc(queue_bytes: usize) -> Result<*mut u8, ()> {
    const PAGESIZE: usize = 4096;
    assert!(queue_bytes & (PAGESIZE - 1) == 0);
    let pages = queue_bytes / PAGESIZE;

    // choose an arbitrary address to store the queue in...
    // Because this is hardcoded, we can only alloc one queue

    let addr = 0x69_0000_0000 as *mut u8;
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
    return Ok(addr);
}

pub fn queue_setup(dev: &mut VirtDevice, queue_num: u32) -> Result<QueueBuf, ()> {
    let max_items = unsafe {
        dev.queue_sel.write(queue_num);
        let max_items = dev.queue_num_max.read();
        if max_items == 0 {
            return Err(());
        }
        max_items
    };
    let queue_sz = core::cmp::min(max_items as usize, 256);
    let desc_sz = 16 * queue_sz;
    let avail_sz = 6 + 2 * queue_sz;
    let used_sz = 6 + 4 * queue_sz;

    fn align(s: usize) -> usize {
        const PAGESIZE: usize = 4096;
        let pages = (s + (PAGESIZE - 1)) / PAGESIZE;
        return pages * PAGESIZE;
    }
    let queue_bytes = align(desc_sz + avail_sz) + align(used_sz);
    let buf = queue_alloc(queue_bytes)?;

    return Ok(QueueBuf {
        size: queue_sz.try_into().unwrap(),
        buf,
    });
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
        device.status.write(DeviceStatus::ACKNOWLEDGE as u32);
        device.status.write(DeviceStatus::DRIVER as u32);

        // negotiate features
        device.host_feauture_sel.write(0);
        let features_low = DeviceFeaturesLow::from_bits_retain(device.host_features.read());
        assert!(features_low.intersects(DeviceFeaturesLow::NINEP_TAGGED));
        device.guest_feauture_sel.write(0);
        device
            .guest_feautures
            .write(DeviceFeaturesLow::NINEP_TAGGED.bits());
        device.status.write(DeviceStatus::FEATURES_OK as u32);
        assert_eq!(device.status.read(), DeviceStatus::FEATURES_OK as u32);

        for i in 0..16 {
            println!("setup queue {i}");
            queue_setup(device, i).unwrap();
        }

        todo!()

        // finish device initialization
    }
}
