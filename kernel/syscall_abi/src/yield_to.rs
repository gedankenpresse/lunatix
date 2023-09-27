//! Definitions for the `yield_to` syscall

use crate::{CAddr, NoValue, RawSyscallArgs, SyscallBinding, SyscallResult};

pub struct YieldTo;

#[derive(Debug, Eq, PartialEq)]
pub struct YieldToArgs {
    /// The task capability to which execution should be yielded.
    pub task: CAddr,
}

impl SyscallBinding for YieldTo {
    const SYSCALL_NO: usize = 11;
    type CallArgs = YieldToArgs;
    type Return = SyscallResult<NoValue>;
}

impl From<YieldToArgs> for RawSyscallArgs {
    fn from(value: YieldToArgs) -> Self {
        [value.task, 0, 0, 0, 0, 0, 0]
    }
}

impl From<RawSyscallArgs> for YieldToArgs {
    fn from(value: RawSyscallArgs) -> Self {
        Self { task: value[0] }
    }
}
