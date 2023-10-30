use crate::syscalls::syscall;
use core::fmt::{self, Write};
use syscall_abi::debug::{DebugLog, DebugLogArgs};
use syscall_abi::debug::{DebugPutc, DebugPutcArgs};

pub fn print(s: &str) {
    const REG_SIZE: usize = core::mem::size_of::<usize>();
    const BUF_SIZE: usize = REG_SIZE * 6;
    for chunk in s.as_bytes().chunks(BUF_SIZE) {
        syscall_writeslice(chunk);
    }
}

pub fn put_c(c: char) {
    syscall_putc(c)
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    SyscallWriter {}.write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::syscalls::_print(format_args!($($arg)*)));
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
        print(s);
        Ok(())
    }
}

pub(crate) fn syscall_writeslice(s: &[u8]) {
    let mut bytes = [0; 48];
    assert!(s.len() <= bytes.len());
    bytes[0..s.len()].copy_from_slice(s);

    syscall::<DebugLog>(DebugLogArgs {
        len: s.len(),
        byte_slice: bytes,
    })
    .unwrap();
}

pub(crate) fn syscall_putc(c: char) {
    syscall::<DebugPutc>(DebugPutcArgs(c)).unwrap();
}
