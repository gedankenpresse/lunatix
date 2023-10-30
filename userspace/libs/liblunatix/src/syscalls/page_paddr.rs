use crate::syscalls::raw_syscall;
use syscall_abi::{CAddr, FromRawSysResponse, SyscallResult};

pub fn page_paddr(page: CAddr) -> SyscallResult<usize> {
    const PADDR: usize = 21;
    let res = raw_syscall(PADDR, page, 0, 0, 0, 0, 0, 0);
    SyscallResult::from_response(res)
}
