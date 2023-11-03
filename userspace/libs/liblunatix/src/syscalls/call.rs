use crate::syscalls::syscall;
use syscall_abi::call::{Call, CallArgs, NUM_DATA_REGS};
use syscall_abi::{CAddr, IpcTag, SyscallResult, SyscallReturnData};

pub fn call(
    cap: CAddr,
    label: usize,
    caps: &[CAddr],
    data: &[usize],
) -> SyscallResult<SyscallReturnData> {
    assert!(caps.len() + data.len() <= NUM_DATA_REGS);

    let arg = |i: usize| {
        if i < caps.len() {
            caps[i].into()
        } else if i - caps.len() < data.len() {
            data[i - caps.len()]
        } else {
            0
        }
    };

    syscall::<Call>(CallArgs {
        target: cap,
        tag: IpcTag::from_parts(label, caps.len() as u8, data.len() as u8),
        raw_args: [arg(0), arg(1), arg(2), arg(3), arg(4)],
    })
}
