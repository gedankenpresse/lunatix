//! Definitions for the `task_assign_vspace` syscall.

use crate::{CAddr, NoValue, RawSyscallArgs, SyscallBinding, SyscallResult};

pub struct TaskAssignVSpace;

#[derive(Debug, Eq, PartialEq)]
pub struct TaskAssignVSpaceArgs {
    /// The CAddr of the vspace which should be assigned to to a task
    pub vspace_addr: CAddr,
    /// The task to which a cspace should be assigned
    pub task_addr: CAddr,
}

impl SyscallBinding for TaskAssignVSpace {
    const SYSCALL_NO: usize = 9;
    type CallArgs = TaskAssignVSpaceArgs;
    type Return = SyscallResult<NoValue>;
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
