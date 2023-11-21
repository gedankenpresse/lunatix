use syscall_abi::destroy::{Destroy, DestroyArgs};
use syscall_abi::{CAddr, NoValue, SyscallResult};

use crate::syscalls::syscall;

pub fn destroy(cap: CAddr) -> SyscallResult<NoValue> {
    syscall::<Destroy>(DestroyArgs { caddr: cap })
}
