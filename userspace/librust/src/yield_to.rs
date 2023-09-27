use crate::syscalls::syscall;
use syscall_abi::yield_to::{YieldTo, YieldToArgs};
use syscall_abi::{CAddr, NoValue, SyscallResult};

pub fn yield_to(task: CAddr) -> SyscallResult<NoValue> {
    syscall::<YieldTo>(YieldToArgs { task })
}
