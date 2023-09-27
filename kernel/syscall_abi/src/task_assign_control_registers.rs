//! Definitions for the `task_assign_control_registers` syscall.

use crate::{CAddr, NoValue, RawSyscallArgs, SyscallBinding, SyscallResult};

pub struct TaskAssignControlRegisters;

#[derive(Debug, Eq, PartialEq)]
pub struct TaskAssignControlRegistersArgs {
    /// The task to which a cspace should be assigned
    pub task_addr: CAddr,
    /// The value to which the program counter of the task should be set.
    pub pc: usize,
    /// The value to which the stack pointer of the task should be set.
    pub sp: usize,
    /// The frame pointer to which the stack pointer of the task should be set.
    pub fp: usize,
    /// The global pointer to which the stack pointer of the task should be set.
    pub gp: usize,
}

impl SyscallBinding for TaskAssignControlRegisters {
    const SYSCALL_NO: usize = 10;
    type CallArgs = TaskAssignControlRegistersArgs;
    type Return = SyscallResult<NoValue>;
}

impl From<TaskAssignControlRegistersArgs> for RawSyscallArgs {
    fn from(value: TaskAssignControlRegistersArgs) -> Self {
        [
            value.task_addr,
            value.pc,
            value.sp,
            value.fp,
            value.gp,
            0,
            0,
        ]
    }
}

impl From<RawSyscallArgs> for TaskAssignControlRegistersArgs {
    fn from(value: RawSyscallArgs) -> Self {
        Self {
            task_addr: value[0],
            pc: value[1],
            sp: value[2],
            fp: value[3],
            gp: value[4],
        }
    }
}
