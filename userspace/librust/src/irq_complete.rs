use crate::syscalls::syscall;
use syscall_abi::irq_complete::{IrqComplete, IrqCompleteArgs};
use syscall_abi::{CAddr, NoValue, SyscallResult};

pub fn irq_complete(irq_addr: CAddr) -> SyscallResult<NoValue> {
    syscall::<IrqComplete>(IrqCompleteArgs { irq_addr })
}
