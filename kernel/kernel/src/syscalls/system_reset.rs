use syscall_abi::system_reset::{ResetReason, ResetType, SystemResetArgs};

pub(super) fn sys_system_reset(args: SystemResetArgs) -> ! {
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
