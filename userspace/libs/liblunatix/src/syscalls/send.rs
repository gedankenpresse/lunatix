use crate::syscalls::syscall;
use syscall_abi::send::{SendArgs, NUM_DATA_REGS};
use syscall_abi::{CAddr, IpcTag, NoValue, SyscallResult};

pub fn send(cap: CAddr, label: usize, caps: &[CAddr], data: &[usize]) -> SyscallResult<NoValue> {
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

    syscall::<syscall_abi::send::Send>(SendArgs {
        target: cap,
        tag: IpcTag::from_parts(label, caps.len() as u8, data.len() as u8),
        raw_args: [arg(0), arg(1), arg(2), arg(3), arg(4)],
    })
}
