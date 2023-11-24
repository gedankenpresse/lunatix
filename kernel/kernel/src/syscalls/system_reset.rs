use crate::sched::Schedule;
use crate::syscalls::handler_trait::SyscallHandler;
use crate::syscalls::SyscallContext;
use crate::KernelContext;
use syscall_abi::system_reset::{ResetReason, ResetType, SystemReset};
use syscall_abi::SyscallBinding;

pub(super) struct SystemResetHandler;

impl SyscallHandler for SystemResetHandler {
    type Syscall = SystemReset;

    fn handle(
        &mut self,
        _kernel_ctx: &mut KernelContext,
        _syscall_ctx: &mut SyscallContext<'_, '_, '_>,
        args: <<Self as SyscallHandler>::Syscall as SyscallBinding>::CallArgs,
    ) -> (
        Schedule,
        <<Self as SyscallHandler>::Syscall as SyscallBinding>::Return,
    ) {
        log::info!(
            "Performing system reset {:?} because {:?}",
            args.typ,
            args.reason
        );
        sbi::system_reset::system_reset(
            match args.typ {
                ResetType::Shutdown => sbi::system_reset::ResetType::Shutdown,
                ResetType::ColdReboot => sbi::system_reset::ResetType::ColdReboot,
                ResetType::WarmReboot => sbi::system_reset::ResetType::WarmReboot,
            },
            match args.reason {
                ResetReason::NoReason => sbi::system_reset::ResetReason::NoReason,
                ResetReason::SystemFailure => sbi::system_reset::ResetReason::SystemFailure,
            },
        )
        .unwrap();
        unreachable!();
    }
}
