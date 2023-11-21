use syscall_abi::copy::{Copy, CopyArgs};
use syscall_abi::{CAddr, NoValue, SyscallResult};

use crate::syscalls::syscall;

pub fn copy(cap: CAddr, target: CAddr) -> SyscallResult<NoValue> {
    syscall::<Copy>(CopyArgs {
        src: cap,
        dst: target,
    })
}
