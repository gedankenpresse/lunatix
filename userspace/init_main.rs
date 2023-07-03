#![no_std]
#![no_main]
use core::arch::asm;
use core::fmt::{self, Write};

fn syscall_putc(c: u8) {
    unsafe { asm!("ecall", in("x10") c) }
}

pub fn print(s: &str) {
    for c in s.bytes() {
        syscall_putc(c);
    }
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    SyscallWriter {}.write_fmt(args).unwrap();
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

/// Dummy struct that makes converting [`fmt::Arguments`] easier to convert to strings
/// by offloading that to the [`Write`] trait.
struct SyscallWriter {}

impl Write for SyscallWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        // call into sbi firmware to write a each character to its output console
        for &char in s.as_bytes() {
            syscall_putc(char);
        }
        Ok(())
    }
}

#[no_mangle]
fn _start() {
    main();
}

static MESSAGE: &'static str = "This is a userspace message!";

fn main() {
    print("hello word!");
    print(MESSAGE);
}

use core::panic::PanicInfo;
#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    println!("panic {}", info);
    loop {}
}
