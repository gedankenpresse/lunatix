use crate::sched::Schedule;
use crate::syscalls::handler_trait::SyscallHandler;
use crate::syscalls::SyscallContext;
use crate::KernelContext;
use syscall_abi::r#yield::Yield;
use syscall_abi::{NoValue, SyscallBinding};

pub(super) struct YieldHandler;

impl SyscallHandler for YieldHandler {
    type Syscall = Yield;

    fn handle(
        &mut self,
        _kernel_ctx: &mut KernelContext,
        _syscall_ctx: &mut SyscallContext<'_, '_, '_, '_>,
        _args: <<Self as SyscallHandler>::Syscall as SyscallBinding>::CallArgs,
    ) -> (
        Schedule,
        <<Self as SyscallHandler>::Syscall as SyscallBinding>::Return,
    ) {
        (Schedule::RunInit, Ok(NoValue))
    }
}
