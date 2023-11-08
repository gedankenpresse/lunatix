use liblunatix::prelude::{
    syscall_abi::{identify::CapabilityVariant, MapFlags},
    CAddr,
};

use virtio::{DescriptorFlags, DeviceId, VirtDevice, VirtQ, VirtQMsgBuf};

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
    fn get(&self) -> T {
        T::from_le(self.0)
    }

    fn set(&mut self, t: T) {
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

impl CtrlHeader {
    fn new(req: CtrlType) -> Self {
        Self {
            typ: LE::new(req as u32),
            ..Default::default()
        }
    }
}

#[repr(C)]
#[derive(Default, Debug)]
struct Rect {
    x: LE<u32>,
    y: LE<u32>,
    width: LE<u32>,
    height: LE<u32>,
}

const MAX_SCANOUTS: usize = 16;

#[repr(C)]
#[derive(Default, Debug)]
struct Display {
    rect: Rect,
    enabled: LE<u32>,
    flags: LE<u32>,
}

#[allow(non_camel_case_types)]
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
#[derive(Default, Debug)]
struct RespDisplayInfo {
    header: CtrlHeader,
    pmodes: [Display; MAX_SCANOUTS],
}

pub struct GpuDriver {
    device: &'static mut VirtDevice,
    ctrl_q: VirtQ,
    cursor_q: VirtQ,
    noti: CAddr,
    irq: CAddr,
}

fn alloc_msg_buf(mem: CAddr, vspace: CAddr, addr: *mut u8) -> VirtQMsgBuf {
    let page1 = caddr_alloc::alloc_caddr();
    liblunatix::ipc::mem::derive(mem, page1, CapabilityVariant::Page, None).unwrap();
    liblunatix::ipc::page::map_page(
        page1,
        vspace,
        mem,
        addr as usize,
        MapFlags::READ | MapFlags::WRITE,
    )
    .unwrap();

    return VirtQMsgBuf {
        buf: unsafe { core::slice::from_raw_parts_mut(addr, 4096) },
        page: page1,
        paddr: liblunatix::ipc::page::get_paddr(page1).unwrap(),
    };
}

fn gpu_do_request<'r, T, R>(
    driver: &mut GpuDriver,
    req: T,
    req_buf: &mut VirtQMsgBuf,
    res_buf: &'r mut VirtQMsgBuf,
) -> &'r R {
    let (num2, desc2) = driver.ctrl_q.get_free_descriptor().unwrap();
    desc2.address = res_buf.paddr as u64;
    desc2.next = 0;
    desc2.flags = DescriptorFlags::WRITE as u16;
    desc2.length = core::mem::size_of::<R>() as u32;
    let (num, desc) = driver.ctrl_q.get_free_descriptor().unwrap();
    desc.address = req_buf.paddr as u64;
    desc.next = num2 as u16;
    desc.flags = DescriptorFlags::NEXT as u16;
    desc.length = core::mem::size_of::<T>() as u32;
    unsafe { req_buf.buf.as_mut_ptr().cast::<T>().write(req) }

    driver.ctrl_q.avail.insert_request(num as u16);
    driver.device.notify(0);
    liblunatix::syscalls::wait_on(driver.noti).unwrap();
    liblunatix::ipc::irq::irq_complete(driver.irq).unwrap();

    unsafe { res_buf.buf.as_ptr().cast::<R>().as_ref().unwrap() }
}

pub fn init_gpu_driver(mem: CAddr, vspace: CAddr, devmem: CAddr, irq_control: CAddr) {
    liblunatix::ipc::devmem::devmem_map(devmem, mem, vspace, VIRTIO_DEVICE, VIRTIO_DEVICE_LEN)
        .unwrap();
    let mut driver = unsafe {
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

        let ctrl_q =
            virtio::queue_setup(device, 0, mem, vspace, 0x34_0000_0000 as *mut u8).unwrap();
        let cursor_q =
            virtio::queue_setup(device, 1, mem, vspace, 0x35_0000_0000 as *mut u8).unwrap();

        device.finish_setup(status);
        GpuDriver {
            device,
            ctrl_q,
            cursor_q,
            noti: irq_notif,
            irq,
        }
    };
    log::info!("got driver!");
    let mut req_buf = alloc_msg_buf(mem, vspace, 0x36_000_0000 as *mut u8);
    let mut res_buf = alloc_msg_buf(mem, vspace, 0x37_000_0000 as *mut u8);

    let res: &RespDisplayInfo = gpu_do_request(
        &mut driver,
        CtrlHeader::new(CtrlType::CMD_GET_DISPLAY_INFO),
        &mut req_buf,
        &mut res_buf,
    );

    let ctype: CtrlType = res.header.typ.get().try_into().unwrap();
    assert_eq!(
        ctype,
        CtrlType::RESP_OK_DISPLAY_INFO,
        "mismatching ctrl types"
    );
    log::info!("resp: {:?}", &res.header);
    log::info!("resp: {:?}", &res.pmodes[0]);
    log::info!("resp: {:?}", &res.pmodes[1]);
}
