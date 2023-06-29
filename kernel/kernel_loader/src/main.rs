//! An elf binary to setup virtual memory and load the kernel in high address ranges
#![no_std]
#![no_main]

mod allocator;
mod logging;
mod virtmem;

#[path = "arch/riscv64imac/mod.rs"]
mod arch;
mod elfloader;

use crate::allocator::BumpAllocator;
use crate::elfloader::KernelLoader;
use crate::logging::KernelLogger;
use crate::virtmem::PageTable;
use ::elfloader::ElfBinary;
use core::panic::PanicInfo;
use log::Level;

static LOGGER: KernelLogger = KernelLogger::new(Level::Debug);

const KERNEL_BIN: &[u8] =
    include_bytes!("../../../target/riscv64imac-unknown-none-elf/debug/kernel");

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    // print panic message
    log::error!("!!! Kernel Loader Panic !!!\n  {}", info);
    loop {}
}

/// The entry point of the loader that is called by U-Boot
#[no_mangle]
pub extern "C" fn _start(argc: u32, argv: *const *const core::ffi::c_char) -> ! {
    LOGGER.install().expect("Could not install logger");
    log::info!("Hello World from Kernel Loader");

    const MEM_START: usize = 0x81400000 + 0x1000000;
    let mut allocator =
        unsafe { BumpAllocator::new(MEM_START as *mut u8, (MEM_START + 0x20000000) as *mut u8) };

    let root_table = unsafe {
        PageTable::empty(&mut allocator)
            .expect("Could not setup root pagetable")
            .as_mut()
            .unwrap()
    };

    let mut kernel_loader = KernelLoader::new(allocator, root_table);
    let binary = ElfBinary::new(KERNEL_BIN).expect("Could not load kernel as elf object");
    binary
        .load(&mut kernel_loader)
        .expect("Could not load the kernel elf binary into memory");
    let entry_point = binary.entry_point();

    unreachable!()
}
