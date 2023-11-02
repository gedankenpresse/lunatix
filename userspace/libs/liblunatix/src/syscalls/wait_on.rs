use crate::syscalls::syscall;
use syscall_abi::wait_on::{WaitOn, WaitOnArgs};
use syscall_abi::{CAddr, NoValue, SyscallResult};

pub fn wait_on(notification: CAddr) -> SyscallResult<NoValue> {
    syscall::<WaitOn>(WaitOnArgs { notification })
}
