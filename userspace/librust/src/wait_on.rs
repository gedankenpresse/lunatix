use crate::syscalls::syscall;
use syscall_abi::wait_on::{WaitOn, WaitOnArgs};
use syscall_abi::{CAddr, SyscallResult};

pub fn wait_on(notification: CAddr) -> SyscallResult<usize> {
    syscall::<WaitOn>(WaitOnArgs { notification })
}
