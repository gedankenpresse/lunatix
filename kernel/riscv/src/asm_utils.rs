//! Utilities for running low-level code that is easier to implement in assembly
//!
//! Exactly relates to `./asm/asm_utils.S`.

#[allow(dead_code)]
extern "C" {
    pub fn read_sstatus() -> usize;
    pub fn wait_for_interrupt();
    pub fn wfi_spin() -> !;
}
