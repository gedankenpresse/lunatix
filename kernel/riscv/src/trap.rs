//! Data Structures for handling trap information

use crate::cpu::{SStatusFlags, TrapEvent};

/// Context information about a trap that was triggered on a RISC-V CPU.
#[repr(C)]
#[derive(Debug, Eq, PartialEq)]
pub struct TrapInfo {
    /// The exception program counter.
    ///
    /// This is the program counter at the point at which the trap was triggered.
    /// Essentially, the program counter of the interrupted code.
    pub epc: usize,

    /// The event that caused the trap to trigger.
    pub cause: TrapEvent,

    /// Supervisor bad address or instruction data.
    ///
    /// If the `cause` field indicates that the cpu encountered a bad instruction or tried to access a bad memory
    /// address, this field holds that bad instruction or bad address.
    /// However, this value is very specific to the instruction cause so care should be taken when interpreting it.
    pub stval: u64,

    /// Information about the execution conditions under which a trap was triggered.
    pub status: SStatusFlags,
}

impl TrapInfo {
    /// Construct an instance by reading the values that are currently stored in the corresponding CPU registers
    pub fn from_current_regs() -> Self {
        Self {
            epc: crate::cpu::Sepc::read(),
            cause: crate::cpu::Scause::read(),
            stval: crate::cpu::StVal::read(),
            status: crate::cpu::SStatus::read(),
        }
    }
}
