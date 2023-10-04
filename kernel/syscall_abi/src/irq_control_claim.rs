//! Definitions for the `irq_control_claim` syscall.

use crate::{CAddr, NoValue, RawSyscallArgs, SyscallBinding, SyscallResult};

pub struct IrqControlClaim;

#[derive(Debug, Eq, PartialEq)]
pub struct IrqControlClaimArgs {
    /// The IRQ-Control capability from which an interrupt line should be claimed
    pub irq_control_addr: CAddr,
    /// The interrupt line that should be claimed
    pub interrupt_line: usize,
    /// The target slot into which an IRQ capability should be placed
    pub irq_addr: CAddr,
    /// The notification capability which should handle IRQs on this interrupt line
    pub notification_addr: CAddr,
}

impl SyscallBinding for IrqControlClaim {
    const SYSCALL_NO: usize = 13;
    type CallArgs = IrqControlClaimArgs;
    type Return = SyscallResult<NoValue>;
}

impl From<IrqControlClaimArgs> for RawSyscallArgs {
    fn from(value: IrqControlClaimArgs) -> Self {
        [
            value.irq_control_addr,
            value.interrupt_line,
            value.irq_addr,
            value.notification_addr,
            0,
            0,
            0,
        ]
    }
}

impl From<RawSyscallArgs> for IrqControlClaimArgs {
    fn from(value: RawSyscallArgs) -> Self {
        Self {
            irq_control_addr: value[0],
            interrupt_line: value[1],
            irq_addr: value[2],
            notification_addr: value[3],
        }
    }
}
