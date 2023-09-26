//! Definitions for the `yield_to` syscall

use crate::generic_return::GenericReturn;
use crate::{CAddr, RawSyscallArgs, RawSyscallReturn, SyscallBinding};

pub struct YieldTo;

#[derive(Debug, Eq, PartialEq)]
pub struct YieldToArgs {
    /// The task capability to which execution should be yielded.
    pub task: CAddr,
}

#[derive(Debug, Eq, PartialEq)]
#[repr(usize)]
pub enum YieldToReturn {
    Success = 0,
    InvalidTaskAddr = 1,
    UnsupportedSyscall = usize::MAX,
}

impl SyscallBinding for YieldTo {
    const SYSCALL_NO: usize = 11;
    type CallArgs = YieldToArgs;
    type Return = YieldToReturn;
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

impl From<YieldToReturn> for RawSyscallReturn {
    fn from(value: YieldToReturn) -> Self {
        match value {
            YieldToReturn::Success => [0, 0],
            YieldToReturn::InvalidTaskAddr => [1, 0],
            YieldToReturn::UnsupportedSyscall => [usize::MAX, 0],
        }
    }
}

impl From<RawSyscallReturn> for YieldToReturn {
    fn from(value: RawSyscallReturn) -> Self {
        match value[0] {
            0 => Self::Success,
            1 => Self::InvalidTaskAddr,
            usize::MAX => Self::UnsupportedSyscall,
            _ => panic!("unknown return; handle this better"),
        }
    }
}

impl From<YieldToReturn> for GenericReturn {
    fn from(value: YieldToReturn) -> Self {
        match value {
            YieldToReturn::Success => Self::Success,
            YieldToReturn::InvalidTaskAddr => Self::Error,
            YieldToReturn::UnsupportedSyscall => Self::UnsupportedSyscall,
        }
    }
}
