//! Definitions for the `task_assign_vspace` syscall.

use crate::generic_return::GenericReturn;
use crate::{CAddr, RawSyscallArgs, RawSyscallReturn, SyscallBinding};

pub struct TaskAssignVSpace;

#[derive(Debug, Eq, PartialEq)]
pub struct TaskAssignVSpaceArgs {
    /// The CAddr of the vspace which should be assigned to to a task
    pub vspace_addr: CAddr,
    /// The task to which a cspace should be assigned
    pub task_addr: CAddr,
}

#[derive(Debug, Eq, PartialEq)]
#[repr(usize)]
pub enum TaskAssignVSpaceReturn {
    Success = 0,
    InvalidVSpaceAddr = 1,
    InvalidTaskAddr = 2,
    UnsupportedSyscall = usize::MAX,
}

impl SyscallBinding for TaskAssignVSpace {
    const SYSCALL_NO: usize = 9;
    type CallArgs = TaskAssignVSpaceArgs;
    type Return = TaskAssignVSpaceReturn;
}

impl From<TaskAssignVSpaceArgs> for RawSyscallArgs {
    fn from(value: TaskAssignVSpaceArgs) -> Self {
        [value.vspace_addr, value.task_addr, 0, 0, 0, 0, 0]
    }
}

impl From<RawSyscallArgs> for TaskAssignVSpaceArgs {
    fn from(value: RawSyscallArgs) -> Self {
        Self {
            vspace_addr: value[0],
            task_addr: value[1],
        }
    }
}

impl From<TaskAssignVSpaceReturn> for RawSyscallReturn {
    fn from(value: TaskAssignVSpaceReturn) -> Self {
        match value {
            TaskAssignVSpaceReturn::Success => [0, 0],
            TaskAssignVSpaceReturn::InvalidVSpaceAddr => [1, 0],
            TaskAssignVSpaceReturn::InvalidTaskAddr => [2, 0],
            TaskAssignVSpaceReturn::UnsupportedSyscall => [usize::MAX, 0],
        }
    }
}

impl From<RawSyscallReturn> for TaskAssignVSpaceReturn {
    fn from(value: RawSyscallReturn) -> Self {
        match value[0] {
            0 => Self::Success,
            1 => Self::InvalidVSpaceAddr,
            2 => Self::InvalidTaskAddr,
            usize::MAX => Self::UnsupportedSyscall,
            _ => panic!("unknown return; handle this better"),
        }
    }
}

impl From<TaskAssignVSpaceReturn> for GenericReturn {
    fn from(value: TaskAssignVSpaceReturn) -> Self {
        match value {
            TaskAssignVSpaceReturn::Success => Self::Success,
            TaskAssignVSpaceReturn::UnsupportedSyscall => Self::UnsupportedSyscall,
            _ => Self::Error,
        }
    }
}
