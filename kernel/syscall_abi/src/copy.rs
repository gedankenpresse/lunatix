//! Definitions for the `copy` syscall.

use crate::{CAddr, NoValue, RawSyscallArgs, SyscallBinding, SyscallResult};

pub struct Copy;

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct CopyArgs {
    pub src: CAddr,
    pub dst: CAddr,
}

impl SyscallBinding for Copy {
    const SYSCALL_NO: usize = 20;
    type CallArgs = CopyArgs;
    type Return = SyscallResult<NoValue>;
}

impl From<CopyArgs> for RawSyscallArgs {
    fn from(value: CopyArgs) -> Self {
        [value.src.raw(), value.dst.raw(), 0, 0, 0, 0, 0]
    }
}

impl From<RawSyscallArgs> for CopyArgs {
    fn from(value: RawSyscallArgs) -> Self {
        Self {
            src: CAddr::from_raw(value[0]),
            dst: CAddr::from_raw(value[1]),
        }
    }
}
