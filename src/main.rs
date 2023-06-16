#![no_std]
#![no_main]

#[path = "arch/riscv64imac/mod.rs"]
mod arch;

mod device_drivers;
mod registers;

use core::fmt;
use core::fmt::Write;
use core::panic::PanicInfo;
use device_drivers::uart::Uart;

#[panic_handler]
fn panic_handler(_info: &PanicInfo) -> ! {
    loop {}
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

    // const vga: *mut u8 = 0xb8000 as *mut u8;
    // unsafe {
    //     core::ptr::write_volatile(vga, 0b01001000 as u8);
    //     core::ptr::write_volatile(vga.add(1), 'a' as u8);
    // }
}
