use core::fmt::{self, Write};

use crate::syscalls::print::SyscallWriter;

pub static mut SYS_WRITER: Option<&'static mut dyn core::fmt::Write> = None;

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    match unsafe { SYS_WRITER.as_mut() } {
        Some(w) => w.write_fmt(args).unwrap(),
        None => SyscallWriter {}.write_fmt(args).unwrap(),
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::print::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}
