use core::{fmt::{self, Write}, arch::asm};
use crate::syscalls;

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
    ($($arg:tt)*) => ($crate::print::_print(format_args!($($arg)*)));
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
    const REG_SIZE: usize = core::mem::size_of::<usize>();
    let mut reg_buf: [usize; 6] = [0usize; 6];
    unsafe { 
        let buf: &mut [u8] = core::slice::from_raw_parts_mut(reg_buf.as_mut_ptr().cast(), REG_SIZE * reg_buf.len());
        assert!(s.len() <= buf.len());
        buf[..s.len()].clone_from_slice(s);
    }
    let [a2, a3, a4, a5, a6, a7] = reg_buf;
    syscalls::raw_syscall(syscalls::SYS_DEBUG_LOG, s.len(), a2, a3, a4, a5, a6, a7);
}

pub (crate) fn syscall_putc(c: u8) {
    unsafe { asm!("ecall", in("x10") syscalls::SYS_DEBUG_PUTC, in("x11") c) }
}