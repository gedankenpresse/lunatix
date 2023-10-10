use syscall_abi::{CAddr, FromRawSysResponse, NoValue, SyscallResult};

use crate::syscalls::raw_syscall;

pub fn destroy(cap: CAddr) -> SyscallResult<NoValue> {
    const DESTROY: usize = 19;
    let res = raw_syscall(DESTROY, cap, 0, 0, 0, 0, 0, 0);
    SyscallResult::from_response(res)
}
