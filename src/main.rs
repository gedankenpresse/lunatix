#![no_std]
#![no_main]

#[path = "arch/riscv64imac/mod.rs"]
mod arch;

mod device_drivers;
mod registers;

use crate::arch::trap::TrapFrame;
use crate::device_drivers::shutdown::{ShutdownCode, SifiveShutdown};
use crate::device_drivers::uart::Uart;
use core::cell::UnsafeCell;
use core::fmt;
use core::fmt::Write;
use core::panic::PanicInfo;
use device_drivers::uart::MmUart;
use fdt_rs::base::DevTree;
use thiserror_no_std::private::DisplayAsDisplay;

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    // print panic message
    println!("!!! Kernel Panic !!!");
    println!("  {}", info.as_display());

    // shutdown the device
    let shutdown_device: &mut SifiveShutdown = unsafe { &mut *(0x100000 as *mut SifiveShutdown) };
    unsafe { shutdown_device.shutdown(ShutdownCode::Fail(1)) }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    // 10_0000_0000
    //    1000_0000
    let mut uart = unsafe { Uart::from_ptr(0x1000_0000 as *mut MmUart) };
    uart.write_fmt(args).unwrap();
}

#[no_mangle]
extern "C" fn kernel_main(_hartid: usize, _unused: usize, dtb: *mut u8) {
    // setup context switching
    let mut stack = [0usize; 2048];
    let trap_frame = unsafe { TrapFrame::null_from_stack(&mut stack as *mut usize, stack.len()) };
    unsafe {
        arch::asm_utils::write_sscratch(&trap_frame as *const TrapFrame as usize);
    }
    arch::trap::enable_interrupts();

    // parse device tree from bootloader
    let device_tree = unsafe { DevTree::from_raw_pointer(dtb).unwrap() };
    let mut uart = unsafe { Uart::from_device_tree(&device_tree).unwrap() };
    uart.write_fmt(format_args!("Hello World {}", 42));

    // shut down the machine
    let shutdown_device: &mut SifiveShutdown = unsafe { &mut *(0x100_000 as *mut SifiveShutdown) };
    unsafe { shutdown_device.shutdown(ShutdownCode::Pass) };
}
