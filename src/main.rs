#![no_std]
#![no_main]

#[path = "arch/riscv64imac/mod.rs"]
mod arch;

use core::fmt;
use core::panic::PanicInfo;

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
    const uart: *mut u8 = 0x1000_0000 as *mut u8;
    let str = args.as_str().unwrap();
    for b in str.as_bytes() {
        unsafe {
            core::ptr::write_volatile(uart, *b);
        }
    }
}

#[no_mangle]
extern "C" fn kernel_main(_hartid: usize, _unused: usize, _dtb: *mut u8) {
    println!("Hello World");

    // const vga: *mut u8 = 0xb8000 as *mut u8;
    // unsafe {
    //     core::ptr::write_volatile(vga, 0b01001000 as u8);
    //     core::ptr::write_volatile(vga.add(1), 'a' as u8);
    // }
}
