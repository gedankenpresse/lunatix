use crate::caps::task::TaskExecutionState;
use crate::caps::{Capability, Tag};
use crate::sched::Schedule;
use crate::syscalls::handler_trait::SyscallHandler;
use crate::syscalls::SyscallContext;
use crate::KernelContext;
use derivation_tree::tree::CursorRefMut;
use syscall_abi::yield_to::{TaskStatus, YieldTo};
use syscall_abi::SyscallBinding;

use super::utils;

pub(super) struct YieldToHandler;

impl SyscallHandler for YieldToHandler {
    type Syscall = YieldTo;

    fn handle(
        &mut self,
        kernel_ctx: &mut KernelContext,
        syscall_ctx: &mut SyscallContext<'_, '_, '_, '_>,
        args: <<Self as SyscallHandler>::Syscall as SyscallBinding>::CallArgs,
    ) -> (
        Schedule,
        <<Self as SyscallHandler>::Syscall as SyscallBinding>::Return,
    ) {
        // get basic caps from task
        let task = syscall_ctx.task.get_inner_task().unwrap();
        let mut cspace = task.get_cspace();
        let cspace = cspace.get_shared().unwrap();
        let cspace = cspace.get_inner_cspace().unwrap();

        // get valid memory cap from task
        let target_task_cap = match unsafe { utils::lookup_cap_mut(cspace, args.task, Tag::Task) } {
            Ok(c) => c,
            Err(_e) => return (Schedule::Keep, Err(syscall_abi::SyscallError::InvalidCap)),
        };
        let target_task_ptr = target_task_cap as *mut Capability;
        let target_task = target_task_cap.get_inner_task().unwrap();
        let target_task_state = target_task.state.borrow();

        match target_task_state.execution_state {
            TaskExecutionState::Running => (Schedule::Keep, Ok(TaskStatus::AlreadyRunning)),
            TaskExecutionState::Waiting => (Schedule::Keep, Ok(TaskStatus::Blocked)),
            TaskExecutionState::Idle => (
                Schedule::RunTask(target_task_ptr),
                Ok(TaskStatus::DidExecute),
            ),
            TaskExecutionState::Exited => (Schedule::Keep, Ok(TaskStatus::Exited)),
        }
    }
}
