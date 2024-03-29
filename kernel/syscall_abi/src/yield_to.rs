//! Definitions for the `yield_to` syscall

use crate::{
    back_to_enum, CAddr, RawSyscallArgs, SyscallBinding, SyscallResult, SyscallReturnData,
};

pub struct YieldTo;

#[derive(Debug, Eq, PartialEq)]
pub struct YieldToArgs {
    /// The task capability to which execution should be yielded.
    pub task: CAddr,
}

back_to_enum! {
    #[derive(Debug, Eq, PartialEq)]
    #[repr(usize)]
    pub enum TaskStatus {
        /// The yield resulted in an execution of the target task.
        DidExecute = 0,
        /// Could not yield to the target task because it is blocked.
        Blocked = 1,
        /// Could not yield to the target task because it is already exited.
        Exited = 2,
        /// Could not yield to the target task because it is already running.
        AlreadyRunning = 3,
    }
}

impl SyscallBinding for YieldTo {
    const SYSCALL_NO: usize = 11;
    type CallArgs = YieldToArgs;
    type Return = SyscallResult<TaskStatus>;
}

impl From<YieldToArgs> for RawSyscallArgs {
    fn from(value: YieldToArgs) -> Self {
        [value.task.into(), 0, 0, 0, 0, 0, 0]
    }
}

impl From<RawSyscallArgs> for YieldToArgs {
    fn from(value: RawSyscallArgs) -> Self {
        Self {
            task: value[0].into(),
        }
    }
}

impl Into<SyscallReturnData> for TaskStatus {
    fn into(self) -> SyscallReturnData {
        [self as usize, 0, 0, 0, 0, 0, 0]
    }
}

impl TryFrom<SyscallReturnData> for TaskStatus {
    type Error = ();

    fn try_from(value: SyscallReturnData) -> Result<Self, Self::Error> {
        Self::try_from(value[0])
    }
}
