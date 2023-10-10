use crate::syscalls::send;
use syscall_abi::{CAddr, FromRawSysResponse, NoValue, SyscallResult};

pub fn irq_complete(irq_addr: CAddr) -> SyscallResult<NoValue> {
    const COMPLETE: usize = 0;
    SyscallResult::from_response(send(irq_addr, COMPLETE, 0, 0, 0, 0, 0))
}
