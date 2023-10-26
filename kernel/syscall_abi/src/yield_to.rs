//! Definitions for the `yield_to` syscall

use crate::{CAddr, RawSyscallArgs, SyscallBinding, SyscallResult};

macro_rules! back_to_enum {
    ($(#[$meta:meta])* $vis:vis enum $name:ident {
        $($(#[$vmeta:meta])* $vname:ident $(= $val:expr)?,)*
    }) => {
        $(#[$meta])*
        $vis enum $name {
            $($(#[$vmeta])* $vname $(= $val)?,)*
        }

        impl core::convert::From<usize> for $name {
            fn from(v: usize) -> Self {
                match v {
                    $(x if x == $name::$vname as usize => $name::$vname,)*
                    _ => panic!(),
                }
            }
        }
    }
}

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
        [value.task, 0, 0, 0, 0, 0, 0]
    }
}

impl From<RawSyscallArgs> for YieldToArgs {
    fn from(value: RawSyscallArgs) -> Self {
        Self { task: value[0] }
    }
}

impl From<TaskStatus> for usize {
    fn from(value: TaskStatus) -> Self {
        value as usize
    }
}
