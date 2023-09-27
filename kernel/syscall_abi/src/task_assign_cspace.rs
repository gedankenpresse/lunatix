//! Definitions for the `task_assign_cspace` syscall.

use crate::{CAddr, NoValue, RawSyscallArgs, SyscallBinding, SyscallResult};

pub struct TaskAssignCSpace;

#[derive(Debug, Eq, PartialEq)]
pub struct TaskAssignCSpaceArgs {
    /// The CAddr of the cspace which should be assigned to to a task
    pub cspace_addr: CAddr,
    /// The task to which a cspace should be assigned
    pub task_addr: CAddr,
}

impl SyscallBinding for TaskAssignCSpace {
    const SYSCALL_NO: usize = 8;
    type CallArgs = TaskAssignCSpaceArgs;
    type Return = SyscallResult<NoValue>;
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
