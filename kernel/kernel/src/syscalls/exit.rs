use crate::caps::task::TaskExecutionState;
use crate::caps::Task;
use crate::sched::Schedule;
use crate::syscalls::handler_trait::SyscallHandler;
use crate::syscalls::SyscallContext;
use crate::KernelContext;
use core::ops::DerefMut;
use syscall_abi::exit::Exit;
use syscall_abi::{NoValue, SyscallBinding};

pub(super) struct ExitHandler;

impl SyscallHandler for ExitHandler {
    type Syscall = Exit;

    fn handle(
        &mut self,
        _kernel_ctx: &mut KernelContext,
        syscall_ctx: &mut SyscallContext<'_, '_, '_, '_>,
        _args: <<Self as SyscallHandler>::Syscall as SyscallBinding>::CallArgs,
    ) -> (
        Schedule,
        <<Self as SyscallHandler>::Syscall as SyscallBinding>::Return,
    ) {
        let mut task_state = syscall_ctx
            .task
            .get_inner_task()
            .unwrap()
            .state
            .borrow_mut();
        task_state.execution_state = TaskExecutionState::Exited;
        (Schedule::RunInit, Ok(NoValue))
    }
}
