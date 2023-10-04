//! Definitions for the `irq_complete` syscall.

use crate::{CAddr, NoValue, RawSyscallArgs, SyscallBinding, SyscallResult};

pub struct IrqComplete;

#[derive(Debug, Eq, PartialEq)]
pub struct IrqCompleteArgs {
    /// The IRQ capability whose pending interrupt should be claimed
    pub irq_addr: CAddr,
}

impl SyscallBinding for IrqComplete {
    const SYSCALL_NO: usize = 15;
    type CallArgs = IrqCompleteArgs;
    type Return = SyscallResult<NoValue>;
}

impl From<IrqCompleteArgs> for RawSyscallArgs {
    fn from(value: IrqCompleteArgs) -> Self {
        [value.irq_addr, 0, 0, 0, 0, 0, 0]
    }
}

impl From<RawSyscallArgs> for IrqCompleteArgs {
    fn from(value: RawSyscallArgs) -> Self {
        Self { irq_addr: value[0] }
    }
}
