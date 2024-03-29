use syscall_abi::{CAddr, NoValue, SyscallResult};

use crate::syscalls::send;

pub fn irq_control_claim(
    irq_control_addr: CAddr,
    interrupt_line: usize,
    irq_addr: CAddr,
    notification_addr: CAddr,
) -> SyscallResult<NoValue> {
    const CLAIM: usize = 0;
    send(
        irq_control_addr,
        CLAIM,
        &[notification_addr, irq_addr],
        &[interrupt_line],
    )
}
