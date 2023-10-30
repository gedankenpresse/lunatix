use core::fmt;
use core::fmt::Write;

#[cfg(target_arch = "riscv64")]
mod sbi_print;

#[cfg(target_arch = "riscv64")]
pub use sbi_print::SbiWriter as KernelWriter;

#[cfg(target_arch = "x86_64")]
mod x86_64_print;

#[cfg(target_arch = "x86_64")]
pub use x86_64_print::KernelWriter;

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    KernelWriter {}.write_fmt(args).unwrap();
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
