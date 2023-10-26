use crate::syscalls::send;
use syscall_abi::{CAddr, FromRawSysResponse, NoValue, SyscallResult};

pub fn irq_complete(irq_addr: CAddr) -> SyscallResult<NoValue> {
    const COMPLETE: u16 = 0;
    send(irq_addr, COMPLETE, &[], &[0, 0, 0, 0, 0])
}
