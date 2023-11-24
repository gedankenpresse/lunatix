use crate::sched::Schedule;
use crate::syscalls::SyscallContext;
use crate::KernelContext;
use syscall_abi::{IntoRawSysRepsonse, RawSyscallArgs, SyscallBinding};

/// A trait for handling a specific syscall in the most bare-bones way possible.
///
/// Argument handling and task state manipulation is entirely left up to the implementation.
pub(super) trait RawSyscallHandler {
    type Syscall: SyscallBinding;

    fn handle_raw(
        &mut self,
        kernel_ctx: &mut KernelContext,
        syscall_ctx: &mut SyscallContext<'_, '_, '_>,
        raw_args: RawSyscallArgs,
    ) -> Schedule;
}

/// A trait for handling most syscalls.
///
/// The `RawSyscallHandler` auto-implementation on top of this guarantees the following:
/// 1. Decode syscall specific arguments from `RawSyscallArgs` and log them
/// 2. *Execute this handler*
/// 3. Log the result and transform it into `RawSyscallReturn`
/// 4. Write the result into the calling tasks registers
/// 5. Increase the calling tasks program counter
pub(super) trait SyscallHandler {
    type Syscall: SyscallBinding;

    fn handle(
        &mut self,
        kernel_ctx: &mut KernelContext,
        syscall_ctx: &mut SyscallContext<'_, '_, '_>,
        args: <<Self as SyscallHandler>::Syscall as SyscallBinding>::CallArgs,
    ) -> (
        Schedule,
        <<Self as SyscallHandler>::Syscall as SyscallBinding>::Return,
    );
}

impl<Handler: SyscallHandler> RawSyscallHandler for Handler {
    type Syscall = <Handler as SyscallHandler>::Syscall;

    fn handle_raw(
        &mut self,
        kernel_ctx: &mut KernelContext,
        syscall_ctx: &mut SyscallContext<'_, '_, '_>,
        raw_args: RawSyscallArgs,
    ) -> Schedule {
        // parse syscall arguments
        let args = <Handler::Syscall as SyscallBinding>::CallArgs::try_from(raw_args)
            .unwrap_or_else(|_| panic!("could not decode syscall args"));

        // execute the handler
        log::trace!(
            "handling {} syscall with args {:x?}",
            core::any::type_name::<Handler::Syscall>(),
            args
        );
        let (schedule, result) = self.handle(kernel_ctx, syscall_ctx, args);
        log::trace!(
            "{} syscall result is {:x?} with new schedule {:?}",
            core::any::type_name::<Handler::Syscall>(),
            result,
            schedule
        );

        // write the result back to userspace
        let mut task_state = syscall_ctx
            .task
            .get_inner_task()
            .unwrap()
            .state
            .borrow_mut();
        task_state
            .frame
            .write_syscall_return(result.into_response());

        // increase the tasks program counter
        task_state.frame.start_pc = syscall_ctx.trap_info.epc + 4;

        schedule
    }
}
