#![no_std]
#![no_main]

#[path = "arch/riscv64imac/mod.rs"]
mod arch;

mod device_drivers;
mod registers;

use crate::device_drivers::shutdown::{ShutdownCode, SifiveShutdown};
use core::fmt;
use core::fmt::Write;
use core::panic::PanicInfo;
use device_drivers::uart::Uart;

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    // print panic message
    println!("!!! Kernel Panic !!!");
    if let Some(loc) = info.location() {
        println!("  At {}:{}:{}", loc.file(), loc.line(), loc.column());
    }

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
    let uart: &mut Uart = unsafe { &mut *(0x1000_0000 as *mut Uart) };
    uart.write_fmt(args).unwrap();
}

#[no_mangle]
extern "C" fn kernel_main(_hartid: usize, _unused: usize, _dtb: *mut u8) {
    println!("Hello World {}", 42);

    let shutdown_device: &mut SifiveShutdown = unsafe { &mut *(0x100000 as *mut SifiveShutdown) };
    unsafe { shutdown_device.shutdown(ShutdownCode::Pass) };
}
