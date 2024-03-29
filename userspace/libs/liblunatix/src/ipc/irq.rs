use crate::syscalls::send;
use syscall_abi::{CAddr, NoValue, SyscallResult};

pub fn irq_complete(irq_addr: CAddr) -> SyscallResult<NoValue> {
    const COMPLETE: usize = 0;
    send(irq_addr, COMPLETE, &[], &[])
}
