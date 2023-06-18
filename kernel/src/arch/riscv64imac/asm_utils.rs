//! Utilities for running low-level code that is easier to implement in assembly
//!
//! Exactly relates to `./asm/asm_utils.S`.

#[allow(dead_code)]
extern "C" {
    /// Read the `sscratch` register and return its content
    pub fn read_sscratch() -> usize;
    /// Write the given data into the `sscratch register`
    pub fn write_sscratch(_: usize);
    /// Atomically swap the given data into `sscratch` and return the previous content
    pub fn read_write_sscratch(_: usize) -> usize;

    /// Atomically set the given bits in the `sstatus` register to `1` (`sstatus |= <input>`) and return the previous content
    pub fn read_set_sstatus(_: usize) -> usize;
    /// Atomically clear the given bits in the `sstatus` register (`sstatus &= !<input>`) and return the previous content
    pub fn read_clear_sstatus(_: usize) -> usize;
    /// Read the content of the `sstatus` register and return it
    pub fn read_sstatus() -> usize;

    pub fn write_sepc(_: usize);
    pub fn read_sepc() -> usize;

    pub fn read_sie() -> usize;
    pub fn set_sie(_: usize);
    pub fn clear_sie(_: usize);

    pub fn read_sip() -> usize;
    pub fn set_sip(_: usize);
    pub fn clear_sip(_: usize);

    pub fn read_stvec() -> usize;
    pub fn write_stvec(_: usize);
    pub fn read_write_stvec(_: usize) -> usize;

    pub fn read_mstatus() -> usize;
    pub fn wait_for_interrupt();
}
