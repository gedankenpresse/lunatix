use core::fmt::{self, Write};
use crate::syscalls::syscall_writeslice;
use crate::syscalls::syscall_putc;

pub fn print(s: &str) {
    const REG_SIZE: usize = core::mem::size_of::<usize>();
    const BUF_SIZE: usize = REG_SIZE * 6;
    for chunk in s.as_bytes().chunks(BUF_SIZE) {
        syscall_writeslice(chunk);
    }
}

pub fn put_c(c: char) {
    syscall_putc(c as u8)
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    SyscallWriter {}.write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => (librust::print::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => (librust::print!("{}\n", format_args!($($arg)*)));
}

/// Dummy struct that makes converting [`fmt::Arguments`] easier to convert to strings
/// by offloading that to the [`Write`] trait.
struct SyscallWriter {}

impl Write for SyscallWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        print(s);
        Ok(())
    }
}