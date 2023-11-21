use crate::sched::Schedule;
use crate::syscalls::SyscallContext;
use crate::KernelContext;
use syscall_abi::{RawSyscallArgs, SyscallBinding};

/// A trait for handling a specific syscall in the most bare-bones way possible.
///
/// Argument handling and task state manipulation is entirely left up to the implementation.
pub(super) trait RawSyscallHandler {
    type Syscall: SyscallBinding;

    fn handle(
        &mut self,
        kernel_ctx: &mut KernelContext,
        syscall_ctx: &mut SyscallContext<'_, '_, '_, '_>,
        raw_args: RawSyscallArgs,
    ) -> Schedule;
}

/// A trait for handling most syscalls.
///
/// The global syscall handler wraps any implementation to automatically perform the following
/// operations:
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
        syscall_ctx: &mut SyscallContext<'_, '_, '_, '_>,
        args: <<Self as SyscallHandler>::Syscall as SyscallBinding>::CallArgs,
    ) -> (
        Schedule,
        <<Self as SyscallHandler>::Syscall as SyscallBinding>::Return,
    );
}
