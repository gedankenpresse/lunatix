use crate::syscalls::syscall;
use syscall_abi::irq_control_claim::{IrqControlClaim, IrqControlClaimArgs};
use syscall_abi::{CAddr, NoValue, SyscallResult};

pub fn irq_control_claim(
    irq_control_addr: CAddr,
    interrupt_line: usize,
    irq_addr: CAddr,
    notification_addr: CAddr,
) -> SyscallResult<NoValue> {
    syscall::<IrqControlClaim>(IrqControlClaimArgs {
        irq_control_addr,
        interrupt_line,
        irq_addr,
        notification_addr,
    })
}
