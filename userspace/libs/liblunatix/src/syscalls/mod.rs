mod copy;
mod destroy;
mod exit;
mod identify;
mod send;
#[macro_use]
mod print;
mod call;
mod system_reset;
mod wait_on;
mod r#yield;
mod yield_to;

use core::arch::asm;
use syscall_abi::{FromRawSysResponse, RawSyscallReturn, SyscallBinding};

pub use call::call;
pub use copy::copy;
pub use destroy::destroy;
pub use exit::exit;
pub use identify::identify;
pub use print::{_print, print, put_c};
pub use r#yield::r#yield;
pub use send::send;
pub use system_reset::system_reset;
pub use wait_on::wait_on;
pub use yield_to::yield_to;

#[inline(always)]
pub fn raw_syscall(
    syscallno: usize,
    a1: usize,
    a2: usize,
    a3: usize,
    a4: usize,
    a5: usize,
    a6: usize,
    a7: usize,
) -> RawSyscallReturn {
    let mut out0: usize;
    let mut out1: usize;
    let mut out2: usize;
    let mut out3: usize;
    let mut out4: usize;
    let mut out5: usize;
    let mut out6: usize;
    let mut out7: usize;
    unsafe {
        asm!(
        "ecall",
        inout("x10") syscallno => out0,
        inout("x11") a1 => out1,
        inout("x12") a2 => out2,
        inout("x13") a3 => out3,
        inout("x14") a4 => out4,
        inout("x15") a5 => out5,
        inout("x16") a6 => out6,
        inout("x17") a7 => out7,
        );
    }
    return [out0, out1, out2, out3, out4, out5, out6, out7];
}

#[inline(always)]
pub(crate) fn syscall<T>(args: T::CallArgs) -> T::Return
where
    T: SyscallBinding,
{
    let [a1, a2, a3, a4, a5, a6, a7] = args.into();
    let result = raw_syscall(T::SYSCALL_NO, a1, a2, a3, a4, a5, a6, a7);
    T::Return::from_response(result)
}
