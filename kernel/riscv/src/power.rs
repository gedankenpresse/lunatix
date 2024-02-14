//! Power (and poweron) state management
use log::error;
use sbi::system_reset::*;

/// Perform a clean shutdown of the host hardware
#[cfg(target_arch = "riscv64")]
pub fn shutdown() -> ! {
    // gracefully shut down the system
    match system_reset(ResetType::Shutdown, ResetReason::NoReason) {
        Ok(_) => unreachable!(),
        Err(e) => error!("shutdown error: {}", e),
    };

    poweroff_fallback();
}

/// Perform an erroring shutdown of the host hardware
#[cfg(target_arch = "riscv64")]
#[inline(always)]
pub fn abort() -> ! {
    match system_reset(ResetType::Shutdown, ResetReason::SystemFailure) {
        Ok(_) => unreachable!(),
        Err(e) => error!("abort error: {}", e),
    };

    poweroff_fallback()
}

#[cfg(target_arch = "riscv64")]
fn poweroff_fallback() -> ! {
    // fall back to legacy shutdown
    sbi::legacy::shutdown();

    // just to make sure the hart never executes anything again, spin indefinitely
    #[allow(unreachable_code)]
    unsafe {
        crate::wfi_spin()
    }
}
