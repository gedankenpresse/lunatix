use crate::println;
use core::arch::asm;
use syscall_abi::send::{Send, SendArgs, NUM_DATA_REGS};
use syscall_abi::{CAddr, NoValue};
use syscall_abi::{FromRawSysResponse, SyscallResult};
use syscall_abi::{IpcTag, SyscallBinding};

pub fn send(cap: CAddr, label: usize, caps: &[CAddr], data: &[usize]) -> SyscallResult<NoValue> {
    assert_eq!(caps.len() + data.len(), NUM_DATA_REGS);

    let arg = |i: usize| {
        if i < caps.len() {
            caps[i]
        } else {
            data[i - caps.len()]
        }
    };

    syscall::<Send>(SendArgs {
        target: cap,
        tag: IpcTag::from_parts(label, caps.len() as u8, data.len() as u8),
        raw_args: [arg(0), arg(1), arg(2), arg(3), arg(4)],
    })
}

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
) -> [usize; 2] {
    let mut out0: usize;
    let mut out1: usize;
    unsafe {
        asm!(
            "ecall",
            inout("x10") syscallno => out0,
            inout("x11") a1 => out1,
            in("x12") a2,
            in("x13") a3,
            in("x14") a4,
            in("x15") a5,
            in("x16") a6,
            in("x17") a7,
        );
    }
    return [out0, out1];
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
