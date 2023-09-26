//! Definitions for the `task_assign_cspace` syscall.

use crate::generic_return::GenericReturn;
use crate::{CAddr, RawSyscallArgs, RawSyscallReturn, SyscallBinding};

pub struct TaskAssignCSpace;

#[derive(Debug, Eq, PartialEq)]
pub struct TaskAssignCSpaceArgs {
    /// The CAddr of the cspace which should be assigned to to a task
    pub cspace_addr: CAddr,
    /// The task to which a cspace should be assigned
    pub task_addr: CAddr,
}

#[derive(Debug, Eq, PartialEq)]
#[repr(usize)]
pub enum TaskAssignCSpaceReturn {
    Success = 0,
    InvalidCSpaceAddr = 1,
    InvalidTaskAddr = 2,
    UnsupportedSyscall = usize::MAX,
}

impl SyscallBinding for TaskAssignCSpace {
    const SYSCALL_NO: usize = 8;
    type CallArgs = TaskAssignCSpaceArgs;
    type Return = TaskAssignCSpaceReturn;
}

impl From<TaskAssignCSpaceArgs> for RawSyscallArgs {
    fn from(value: TaskAssignCSpaceArgs) -> Self {
        [value.cspace_addr, value.task_addr, 0, 0, 0, 0, 0]
    }
}

impl From<RawSyscallArgs> for TaskAssignCSpaceArgs {
    fn from(value: RawSyscallArgs) -> Self {
        Self {
            cspace_addr: value[0],
            task_addr: value[1],
        }
    }
}

impl From<TaskAssignCSpaceReturn> for RawSyscallReturn {
    fn from(value: TaskAssignCSpaceReturn) -> Self {
        match value {
            TaskAssignCSpaceReturn::Success => [0, 0],
            TaskAssignCSpaceReturn::InvalidCSpaceAddr => [1, 0],
            TaskAssignCSpaceReturn::InvalidTaskAddr => [2, 0],
            TaskAssignCSpaceReturn::UnsupportedSyscall => [usize::MAX, 0],
        }
    }
}

impl From<RawSyscallReturn> for TaskAssignCSpaceReturn {
    fn from(value: RawSyscallReturn) -> Self {
        match value[0] {
            0 => Self::Success,
            1 => Self::InvalidCSpaceAddr,
            2 => Self::InvalidTaskAddr,
            usize::MAX => Self::UnsupportedSyscall,
            _ => panic!("unknown return; handle this better"),
        }
    }
}

impl From<TaskAssignCSpaceReturn> for GenericReturn {
    fn from(value: TaskAssignCSpaceReturn) -> Self {
        match value {
            TaskAssignCSpaceReturn::Success => Self::Success,
            TaskAssignCSpaceReturn::UnsupportedSyscall => Self::UnsupportedSyscall,
            _ => Self::Error,
        }
    }
}
