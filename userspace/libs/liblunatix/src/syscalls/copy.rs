use syscall_abi::{CAddr, FromRawSysResponse, NoValue, SyscallResult};

use crate::syscalls::raw_syscall;

pub fn copy(cap: CAddr, target: CAddr) -> SyscallResult<NoValue> {
    const COPY: usize = 20;
    let res = raw_syscall(COPY, cap.into(), target.into(), 0, 0, 0, 0, 0);
    SyscallResult::from_response(res)
}
