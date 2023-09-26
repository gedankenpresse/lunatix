use crate::syscalls::syscall;
use syscall_abi::yield_to::{YieldTo, YieldToArgs, YieldToReturn};
use syscall_abi::CAddr;

pub fn yield_to(task: CAddr) -> YieldToReturn {
    syscall::<YieldTo>(YieldToArgs { task }).unwrap()
}
