//! Definitions for the `destroy` syscall.

use crate::{CAddr, NoValue, RawSyscallArgs, SyscallBinding, SyscallResult};

pub struct Destroy;

#[derive(Debug, Eq, PartialEq)]
pub struct DestroyArgs {
    pub caddr: CAddr,
}

impl SyscallBinding for Destroy {
    const SYSCALL_NO: usize = 19;
    type CallArgs = DestroyArgs;
    type Return = SyscallResult<NoValue>;
}

impl From<DestroyArgs> for RawSyscallArgs {
    fn from(value: DestroyArgs) -> Self {
        [value.caddr.raw(), 0, 0, 0, 0, 0, 0]
    }
}

impl From<RawSyscallArgs> for DestroyArgs {
    fn from(value: RawSyscallArgs) -> Self {
        Self {
            caddr: CAddr::from_raw(value[0]),
        }
    }
}
