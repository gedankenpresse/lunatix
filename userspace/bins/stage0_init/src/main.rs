#![no_std]
#![no_main]

mod logger;
mod static_once_cell;

use crate::logger::Logger;
use crate::static_once_cell::StaticOnceCell;
use allocators::boundary_tag_alloc::{BoundaryTagAllocator, TagsU32};
use caddr_alloc::CAddrAlloc;
use core::panic::PanicInfo;
use core::sync::atomic::AtomicUsize;
use liblunatix::prelude::syscall_abi::system_reset::{ResetReason, ResetType};
use liblunatix::prelude::syscall_abi::MapFlags;
use liblunatix::prelude::{CAddr, CapabilityVariant};
use log::Level;

/// How many bits the init tasks cspace uses to address its capabilities
const CSPACE_BITS: usize = 7; // capacity = 128

// CAddrs that the kernel sets for us
const CADDR_MEM: CAddr = CAddr::new(1, CSPACE_BITS);
const _CADDR_CSPACE: CAddr = CAddr::new(2, CSPACE_BITS);
const CADDR_VSPACE: CAddr = CAddr::new(3, CSPACE_BITS);
const CADDR_IRQ_CONTROL: CAddr = CAddr::new(4, CSPACE_BITS);
const CADDR_DEVMEM: CAddr = CAddr::new(5, CSPACE_BITS);
const CADDR_ASID_CONTROL: CAddr = CAddr::new(6, CSPACE_BITS);

static LOGGER: Logger = Logger::new(Level::Info);

static CADDR_ALLOC: CAddrAlloc = CAddrAlloc {
    cspace_bits: AtomicUsize::new(CSPACE_BITS),
    cur: AtomicUsize::new(10),
};

#[global_allocator]
static ALLOC: StaticOnceCell<BoundaryTagAllocator<'static, TagsU32>> = StaticOnceCell::new();

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    log::error!("panic {}", info);
    liblunatix::syscalls::system_reset(ResetType::Shutdown, ResetReason::SystemFailure);
}

#[no_mangle]
extern "C" fn _start() {
    LOGGER.install().expect("could not install logger");
    unsafe {
        caddr_alloc::set_global_caddr_allocator(&CADDR_ALLOC);
    }
    ALLOC.get_or_init(|| unsafe { init_allocator(32, 0x10_000 as *mut u8) });
    main();
}

fn main() {
    log::info!("hello from stage0_init")
}

unsafe fn init_allocator(pages: usize, addr: *mut u8) -> BoundaryTagAllocator<'static, TagsU32> {
    const PAGESIZE: usize = 4096;
    for i in 0..pages {
        let page = caddr_alloc::alloc_caddr();
        liblunatix::ipc::mem::derive(CADDR_MEM, page, CapabilityVariant::Page, None).unwrap();
        liblunatix::ipc::page::map_page(
            page,
            CADDR_VSPACE,
            CADDR_MEM,
            addr as usize + i * PAGESIZE,
            MapFlags::READ | MapFlags::WRITE,
        )
        .unwrap();
    }

    let mem = unsafe { core::slice::from_raw_parts_mut(addr, pages * PAGESIZE) };
    mem.fill(0);
    BoundaryTagAllocator::new(mem)
}
