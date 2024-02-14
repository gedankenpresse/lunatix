//! Utilities for running low-level riscv specific code that is implemented in assembly

use core::arch::{asm, global_asm};

/// Put the current hart to sleep until an interrupt wakes it up again.
///
/// Note that this only provides a hint to the hardware implementation to wait until an interrupt *might* need servicing.
/// No interrupt is guaranteed to actually be pending when this function returns.
/// Calling `wait_for_interrupt()` might also influence how the hardware implementation routes interrupts to harts as the hardware might prefer harts that called `wait_for_interrupt()`.
#[inline(always)]
pub fn wait_for_interrupt() {
    unsafe { asm!("wfi", options(nomem, nostack)) }
}

/// Call wait_for_interrupt() in a tight loop.
///
/// This effectively puts the hart to sleep forever without wasting as much power as a busy loop would require.
#[inline(always)]
pub fn wfi_spin() -> ! {
    loop {
        wait_for_interrupt();
    }
}
