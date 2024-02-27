//! Power (and poweron) state management
use log::error;
use sbi::system_reset::*;

/// Perform a clean shutdown of the host hardware
pub fn shutdown() -> ! {
    // gracefully shut down the system
    match system_reset(ResetType::Shutdown, ResetReason::NoReason) {
        Ok(_) => unreachable!(),
        Err(e) => error!("shutdown error: {}", e),
    };

    // fall back to legacy shutdown
    sbi::legacy::shutdown();
}

/// Perform an erroring shutdown of the host hardware
#[cfg(target_arch = "riscv64")]
#[inline(always)]
pub fn abort() -> ! {
    match system_reset(ResetType::Shutdown, ResetReason::SystemFailure) {
        Ok(_) => unreachable!(),
        Err(e) => error!("abort error: {}", e),
    };

    // fall back to legacy shutdown
    sbi::legacy::shutdown();
}
