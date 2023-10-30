use crate::syscalls::syscall;
use syscall_abi::{
    r#yield::{Yield, YieldArgs},
    NoValue, SyscallResult,
};

pub fn r#yield() -> SyscallResult<NoValue> {
    syscall::<Yield>(YieldArgs {})
}
