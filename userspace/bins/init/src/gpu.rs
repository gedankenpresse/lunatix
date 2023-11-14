use core::{alloc::Layout, borrow::Borrow, sync::atomic::AtomicU64};

use alloc::vec;
use liblunatix::{
    prelude::{
        syscall_abi::{identify::CapabilityVariant, MapFlags},
        CAddr,
    },
    println, MemoryPage,
};

use virtio::{DescriptorFlags, DeviceId, VirtDevice, VirtQ, VirtQMsgBuf};

use crate::CSPACE_BITS;

const VIRTIO_DEVICE: usize = 0x10007000;
const VIRTIO_DEVICE_LEN: usize = 0x1000;

/// A macro for creating an enum that supports `TryFrom<usize>` conversion
#[macro_export]
macro_rules! back_to_enum_u32 {
    ($(#[$meta:meta])* $vis:vis enum $name:ident {
        $($(#[$vmeta:meta])* $vname:ident $(= $val:expr)?,)*
    }) => {
        $(#[$meta])*
        $vis enum $name {
            $($(#[$vmeta])* $vname $(= $val)?,)*
        }

        impl core::convert::TryFrom<u32> for $name {
            type Error = ();

            fn try_from(v: u32) -> Result<Self, Self::Error> {
                match v {
                    $(x if x == $name::$vname as u32 => Ok($name::$vname),)*
                    _ => Err(()),
                }
            }
        }
    }
}

back_to_enum_u32! {
    #[allow(non_camel_case_types)]
    #[repr(u32)]
    #[derive(Debug, PartialEq, Eq)]
    enum CtrlType {
        /* 2d commands */
        CMD_GET_DISPLAY_INFO = 0x0100,
        CMD_RESOURCE_CREATE_2D,
        CMD_RESOURCE_UNREF,
        CMD_SET_SCANOUT,
        CMD_RESOURCE_FLUSH,
        CMD_TRANSFER_TO_HOST_2D,
        CMD_RESOURCE_ATTACH_BACKING,
        CMD_RESOURCE_DETACH_BACKING,
        CMD_GET_CAPSET_INFO,
        CMD_GET_CAPSET,
        CMD_GET_EDID,

        /* cursor commands */
        CMD_UPDATE_CURSOR = 0x0300,
        CMD_MOVE_CURSOR,

        /* success responses */
        RESP_OK_NODATA = 0x1100,
        RESP_OK_DISPLAY_INFO,
        RESP_OK_CAPSET_INFO,
        RESP_OK_CAPSET,
        RESP_OK_EDID,

        /* error responses */
        RESP_ERR_UNSPEC = 0x1200,
        RESP_ERR_OUT_OF_MEMORY,
        RESP_ERR_INVALID_SCANOUT_ID,
        RESP_ERR_INVALID_RESOURCE_ID,
        RESP_ERR_INVALID_CONTEXT_ID,
        RESP_ERR_INVALID_PARAMETER,
    }

}
pub trait LittleEndian {
    fn from_le(t: Self) -> Self;
    fn to_le(t: Self) -> Self;
}

impl LittleEndian for u64 {
    fn from_le(t: Self) -> Self {
        u64::from_le(t)
    }

    fn to_le(t: Self) -> Self {
        u64::to_le(t)
    }
}

impl LittleEndian for u32 {
    fn from_le(t: Self) -> Self {
        u32::from_le(t)
    }

    fn to_le(t: Self) -> Self {
        u32::to_le(t)
    }
}

#[repr(transparent)]
#[derive(Clone)]
pub struct LE<T: LittleEndian>(T);

impl<T: LittleEndian + Copy + core::fmt::Debug> core::fmt::Debug for LE<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("LE").field(&T::from_le(self.0)).finish()
    }
}

impl<T: LittleEndian> LE<T> {
    fn new(v: T) -> Self {
        LE(T::to_le(v))
    }
}

impl<T: LittleEndian + Copy> LE<T> {
    pub fn get(&self) -> T {
        T::from_le(self.0)
    }

    pub fn set(&mut self, t: T) {
        self.0 = T::to_le(t)
    }
}

impl<T: LittleEndian + Default> Default for LE<T> {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

#[repr(C)]
#[derive(Default, Debug)]
struct CtrlHeader {
    typ: LE<u32>,
    flags: LE<u32>,
    fence_id: LE<u64>,
    ctx_id: LE<u32>,
    _padding: LE<u32>,
}

#[repr(u32)]
enum HeaderFlags {
    FENCE = 1,
}

struct CtrlHeaderBuilder {
    header: CtrlHeader,
}

impl CtrlHeaderBuilder {
    fn new(req: CtrlType) -> Self {
        Self {
            header: CtrlHeader {
                typ: LE::new(req as u32),
                flags: LE::new(0),
                fence_id: LE::new(0),
                ctx_id: LE::new(0),
                _padding: LE::new(0),
            },
        }
    }

    fn flag(mut self, flag: HeaderFlags) -> Self {
        let cur = self.header.flags.get();
        self.header.flags.set(cur | flag as u32);
        self
    }

    fn fence(mut self, id: u64) -> Self {
        self.header.fence_id.set(id);
        self.flag(HeaderFlags::FENCE)
    }

    fn finish(self) -> CtrlHeader {
        self.header
    }
}

#[repr(C)]
#[derive(Default, Debug, Clone)]
pub struct Rect {
    pub x: LE<u32>,
    pub y: LE<u32>,
    pub width: LE<u32>,
    pub height: LE<u32>,
}

const MAX_SCANOUTS: usize = 16;

#[repr(C)]
#[derive(Default, Debug, Clone)]
pub struct Display {
    pub rect: Rect,
    enabled: LE<u32>,
    flags: LE<u32>,
}

#[allow(non_camel_case_types)]
#[allow(unused)]
#[repr(u32)]
enum ResourceFormats {
    B8G8R8A8_UNORM = 1,
    B8G8R8X8_UNORM = 2,
    A8R8G8B8_UNORM = 3,
    X8R8G8B8_UNORM = 4,

    R8G8B8A8_UNORM = 67,
    X8B8G8R8_UNORM = 68,

    A8B8G8R8_UNORM = 121,
    R8G8B8X8_UNORM = 134,
}

#[repr(C)]
struct ResourceCreate2D {
    header: CtrlHeader,
    resource_id: LE<u32>,
    format: LE<u32>,
    width: LE<u32>,
    height: LE<u32>,
}

#[repr(C)]
struct MemEntry {
    addr: LE<u64>,
    length: LE<u32>,
    padding: LE<u32>,
}

#[repr(C)]
#[allow(unused)]
struct ResourceCreateBacking {
    header: CtrlHeader,
    resource_id: LE<u32>,
    nr_entries: LE<u32>,
    entries: [MemEntry; 254],
}

const _: () = assert!(4096 == core::mem::size_of::<ResourceCreateBacking>());

#[repr(C)]
struct ResourceCreateBackingSingle {
    header: CtrlHeader,
    resource_id: LE<u32>,
    /// HAS TO BE EXACTLY 1
    nr_entries: LE<u32>,
    entry: MemEntry,
}

#[repr(C)]
struct TransferToHost2d {
    header: CtrlHeader,
    rect: Rect,
    offset: LE<u64>,
    resource_id: LE<u32>,
    padding: LE<u32>,
}

#[repr(C)]
struct SetScanout {
    header: CtrlHeader,
    rect: Rect,
    scanout_id: LE<u32>,
    resource_id: LE<u32>,
}

#[repr(C)]
struct ResourceFlush {
    header: CtrlHeader,
    rect: Rect,
    resource_id: LE<u32>,
    padding: LE<u32>,
}

#[repr(C)]
#[derive(Default, Debug)]
struct RespDisplayInfo {
    header: CtrlHeader,
    pmodes: [Display; MAX_SCANOUTS],
}

#[allow(unused)]
pub struct GpuDriver {
    device: &'static mut VirtDevice,
    ctrl_q: VirtQ,
    cursor_q: VirtQ,
    noti: CAddr,
    irq: CAddr,
    req_buf: VirtQMsgBuf,
    res_buf: VirtQMsgBuf,
}

pub struct GpuFramebuffer {
    pub page_cspace: CAddr,
    pub resource_id: u32,
    pub scanout: u32,
    pub width: u32,
    pub height: u32,
    pub buf: &'static mut [u32],
}

fn alloc_msg_buf(mem: CAddr, vspace: CAddr) -> VirtQMsgBuf {
    let region = mmap::allocate_raw(Layout::new::<MemoryPage>()).unwrap();
    let page1 = caddr_alloc::alloc_caddr();
    liblunatix::ipc::mem::derive(mem, page1, CapabilityVariant::Page, None).unwrap();
    liblunatix::ipc::page::map_page(
        page1,
        vspace,
        mem,
        region.start as usize,
        MapFlags::READ | MapFlags::WRITE,
    )
    .unwrap();

    return VirtQMsgBuf {
        buf: unsafe { core::slice::from_raw_parts_mut(region.start, 4096) },
        page: page1,
        paddr: liblunatix::ipc::page::get_paddr(page1).unwrap(),
    };
}

fn gpu_do_request<'r, T, R>(
    driver: &mut GpuDriver,
    req: impl Borrow<T>,
    req_buf: &mut VirtQMsgBuf,
    res_buf: &'r mut VirtQMsgBuf,
) -> &'r R {
    let req = req.borrow();
    let (num2, desc2) = driver.ctrl_q.get_free_descriptor().unwrap();
    desc2.address = res_buf.paddr as u64;
    desc2.next = 0;
    desc2.flags = DescriptorFlags::WRITE as u16;
    let res_length = core::mem::size_of::<R>() as u32;
    assert!(res_length != 0);
    desc2.length = res_length;
    let (num, desc) = driver.ctrl_q.get_free_descriptor().unwrap();
    desc.address = req_buf.paddr as u64;
    desc.next = num2 as u16;
    desc.flags = DescriptorFlags::NEXT as u16;
    let req_length = core::mem::size_of::<T>() as u32;
    assert!(req_length != 0);
    desc.length = req_length;
    unsafe {
        let src_ptr = req as *const T;
        let dest_ptr = req_buf.buf.as_mut_ptr().cast::<T>();
        core::ptr::copy_nonoverlapping(src_ptr, dest_ptr, 1);
    }

    let mut last_used = *driver.ctrl_q.used.idx % driver.ctrl_q.descriptor_table.len() as u16;
    driver.ctrl_q.avail.insert_request(num as u16);
    driver.device.notify(0);

    // TODO: understand why this works.
    // ... Okay, just joking.
    // Sometimes the device interrupts without actually having a new events, so we try to await new events in a loop.
    // In theory it is possible to receive multiple new events, so we handle that in a loop as well.
    // Last, but not least, a single used event can have multiple chained descriptors, so we handle those in a loop as well.
    let mut done = false;
    while !done {
        liblunatix::syscalls::wait_on(driver.noti).unwrap();
        let used_idx = *driver.ctrl_q.used.idx % driver.ctrl_q.descriptor_table.len() as u16;
        while last_used != used_idx {
            let used_elem = driver.ctrl_q.used.ring[last_used as usize];
            let mut idx = used_elem.id as u16;
            let mut desc = &mut driver.ctrl_q.descriptor_table[idx as usize];
            while DescriptorFlags::NEXT as u16 & desc.flags != 0 {
                if num as u16 == idx {
                    done = true;
                }
                idx = desc.next;
                desc.free();
                desc = &mut driver.ctrl_q.descriptor_table[idx as usize];
            }
            if num as u16 == idx {
                done = true;
            }
            desc.free();
            last_used = (last_used + 1) % driver.ctrl_q.descriptor_table.len() as u16;
        }

        liblunatix::ipc::irq::irq_complete(driver.irq).unwrap();
    }

    unsafe { res_buf.buf.as_ptr().cast::<R>().as_ref().unwrap() }
}

fn assert_phys_cont(pages: &[CAddr]) {
    for i in 1..pages.len() {
        let prev = liblunatix::ipc::page::get_paddr(pages[i - 1]).unwrap();
        let cur = liblunatix::ipc::page::get_paddr(pages[i]).unwrap();
        assert_eq!(prev + 4096, cur, "pages are not physically contigous");
    }
}

pub fn init_gpu_driver(mem: CAddr, vspace: CAddr, devmem: CAddr, irq_control: CAddr) -> GpuDriver {
    liblunatix::ipc::devmem::devmem_map(devmem, mem, vspace, VIRTIO_DEVICE, VIRTIO_DEVICE_LEN)
        .unwrap();
    let driver = unsafe {
        let device = VirtDevice::at(VIRTIO_DEVICE as *mut VirtDevice);
        assert_eq!(device.device_id.read(), DeviceId::GPU_DEVICE);
        let mut status = device.init();
        status = device.negotiate_features(status, 0 as u64);

        // setup an irq handler for the virtio device
        let irq_notif = caddr_alloc::alloc_caddr();
        liblunatix::ipc::mem::derive(mem, irq_notif, CapabilityVariant::Notification, None)
            .unwrap();
        let irq = caddr_alloc::alloc_caddr();
        liblunatix::ipc::irq_control::irq_control_claim(irq_control, 0x07, irq, irq_notif).unwrap();

        let ctrl_q = virtio::queue_setup(device, 0, mem, vspace).unwrap();
        let cursor_q = virtio::queue_setup(device, 1, mem, vspace).unwrap();

        device.finish_setup(status);

        let req_buf = alloc_msg_buf(mem, vspace);
        let res_buf = alloc_msg_buf(mem, vspace);
        GpuDriver {
            device,
            ctrl_q,
            cursor_q,
            noti: irq_notif,
            irq,
            req_buf,
            res_buf,
        }
    };
    log::info!("got driver!");
    return driver;
}

impl GpuDriver {
    fn do_request<T, R>(&mut self, req: impl Borrow<T>) -> &R {
        let req_buf = unsafe { ((&mut self.req_buf) as *mut VirtQMsgBuf).as_mut().unwrap() };
        let res_buf = unsafe { ((&mut self.req_buf) as *mut VirtQMsgBuf).as_mut().unwrap() };
        gpu_do_request(self, req, req_buf, res_buf)
    }

    pub fn get_displays(&mut self) -> [Display; 16] {
        let res: &RespDisplayInfo = self.do_request(
            CtrlHeaderBuilder::new(CtrlType::CMD_GET_DISPLAY_INFO)
                .fence(0x1)
                .finish(),
        );

        let ctype: CtrlType = res.header.typ.get().try_into().unwrap();
        assert_eq!(
            ctype,
            CtrlType::RESP_OK_DISPLAY_INFO,
            "mismatching ctrl types"
        );
        return res.pmodes.clone();
    }

    pub fn create_resource(
        &mut self,
        mem: CAddr,
        vspace: CAddr,
        resource_id: u32,
        scanout: u32,
        width: u32,
        height: u32,
    ) -> GpuFramebuffer {
        let bytes = width as usize * height as usize * 4;
        const PAGESIZE: usize = 4096;
        let page_count = (bytes + PAGESIZE - 1) / PAGESIZE;

        let req = ResourceCreate2D {
            header: CtrlHeaderBuilder::new(CtrlType::CMD_RESOURCE_CREATE_2D)
                .fence(0x1234)
                .finish(),
            format: LE::new(ResourceFormats::A8R8G8B8_UNORM as u32),
            height: LE::new(height),
            width: LE::new(width),
            resource_id: LE::new(resource_id),
        };
        let res: &CtrlHeader = self.do_request(req);
        assert_eq!(
            CtrlType::try_from(res.typ.get()).unwrap(),
            CtrlType::RESP_OK_NODATA
        );

        let fb_cspace = caddr_alloc::alloc_caddr();
        const FB_BITS: usize = 10;
        liblunatix::ipc::mem::derive(
            mem,
            fb_cspace,
            CapabilityVariant::CSpace,
            Some(1 << FB_BITS),
        )
        .expect("creating CSpace failed");
        assert_eq!(
            CapabilityVariant::CSpace,
            liblunatix::syscalls::identify(fb_cspace).unwrap()
        );
        let mut pages = vec![];
        for i in 0..page_count {
            let page_addr = CAddr::builder()
                .part(fb_cspace.raw(), CSPACE_BITS)
                .part(i, FB_BITS)
                .finish();
            // println!("{:064b} page cspace addr", page_addr);
            liblunatix::ipc::mem::derive(mem, page_addr, CapabilityVariant::Page, None)
                .expect("failed deriving page");
            pages.push(page_addr);
        }
        assert_phys_cont(&pages);
        let phys_addr = liblunatix::ipc::page::get_paddr(pages[0]).unwrap();
        let fb_region =
            mmap::allocate_raw(Layout::from_size_align(PAGESIZE * page_count, 4096).unwrap())
                .unwrap();
        for (i, page) in pages.iter().enumerate() {
            let addr = unsafe { fb_region.start.add(i * PAGESIZE) };
            liblunatix::ipc::page::map_page(
                *page,
                vspace,
                mem,
                addr as usize,
                MapFlags::READ | MapFlags::WRITE,
            )
            .unwrap();
        }
        let fb_buf = unsafe {
            core::slice::from_raw_parts_mut(
                fb_region.start.cast::<u32>(),
                bytes / core::mem::size_of::<u32>(),
            )
        };

        println!("framebuffer start: 0x{:x} size: 0x{:x}", phys_addr, bytes);
        assert!(bytes >= (width * height * 4) as usize);

        let req = ResourceCreateBackingSingle {
            header: CtrlHeaderBuilder::new(CtrlType::CMD_RESOURCE_ATTACH_BACKING)
                .fence(0x123)
                .finish(),
            resource_id: LE::new(resource_id),
            nr_entries: LE::new(1),
            entry: MemEntry {
                addr: LE::new(phys_addr as u64),
                length: LE::new(width * height * 4),
                padding: LE::new(0),
            },
        };
        let res: &CtrlHeader = self.do_request(req);
        assert_eq!(
            CtrlType::try_from(res.typ.get()).unwrap(),
            CtrlType::RESP_OK_NODATA
        );

        let req = SetScanout {
            header: CtrlHeaderBuilder::new(CtrlType::CMD_SET_SCANOUT)
                .fence(0x1231)
                .finish(),
            resource_id: LE::new(resource_id),
            scanout_id: LE::new(0),
            rect: Rect {
                x: LE::new(0),
                y: LE::new(0),
                width: LE::new(width),
                height: LE::new(height),
            },
        };
        let res: &CtrlHeader = self.do_request(req);
        assert_eq!(
            CtrlType::try_from(res.typ.get()).unwrap(),
            CtrlType::RESP_OK_NODATA
        );

        return GpuFramebuffer {
            page_cspace: fb_cspace,
            resource_id,
            scanout,
            width,
            height,
            buf: fb_buf,
        };
    }

    fn transfer_to_host_2d(&mut self, fb: &GpuFramebuffer, fence: u64) {
        let req = TransferToHost2d {
            header: CtrlHeaderBuilder::new(CtrlType::CMD_TRANSFER_TO_HOST_2D)
                .fence(fence)
                .finish(),
            resource_id: LE::new(fb.resource_id),
            offset: LE::new(0),
            padding: LE::new(0),
            rect: Rect {
                x: LE::new(0),
                y: LE::new(0),
                width: LE::new(fb.width),
                height: LE::new(fb.height),
            },
        };
        let res: &CtrlHeader = self.do_request(req);

        assert_eq!(fence, res.fence_id.get());
        assert_eq!(
            CtrlType::try_from(res.typ.get()).unwrap(),
            CtrlType::RESP_OK_NODATA
        );
    }

    fn flush(&mut self, fb: &GpuFramebuffer, fence: u64) {
        let req = ResourceFlush {
            header: CtrlHeaderBuilder::new(CtrlType::CMD_RESOURCE_FLUSH)
                .fence(fence)
                .finish(),
            resource_id: LE::new(fb.resource_id),
            padding: LE::new(0),
            rect: Rect {
                x: LE::new(0),
                y: LE::new(0),
                width: LE::new(fb.width),
                height: LE::new(fb.height),
            },
        };
        let res: &CtrlHeader = self.do_request(req);
        assert_eq!(fence, res.fence_id.get());
        assert_eq!(
            CtrlType::try_from(res.typ.get()).unwrap(),
            CtrlType::RESP_OK_NODATA
        );
    }

    pub fn draw_resource(&mut self, fb: &GpuFramebuffer) {
        static FENCE_ID: AtomicU64 = AtomicU64::new(0x1000);
        use core::sync::atomic::Ordering;
        self.transfer_to_host_2d(fb, FENCE_ID.fetch_add(1, Ordering::SeqCst));
        self.flush(fb, FENCE_ID.fetch_add(1, Ordering::SeqCst));
    }
}
