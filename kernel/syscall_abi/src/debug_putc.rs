//! Definitions for the `debug_putc` syscall

use crate::{NoValue, RawSyscallArgs, SyscallBinding, SyscallResult};
use core::convert::Infallible;

pub struct DebugPutc;

pub struct DebugPutcArgs(pub char);

impl TryFrom<RawSyscallArgs> for DebugPutcArgs {
    type Error = Infallible;

    fn try_from(value: RawSyscallArgs) -> Result<Self, Self::Error> {
        Ok(Self(*value.first().unwrap() as u8 as char))
    }
}

impl Into<RawSyscallArgs> for DebugPutcArgs {
    fn into(self) -> RawSyscallArgs {
        [self.0 as usize, 0, 0, 0, 0, 0, 0]
    }
}

impl SyscallBinding for DebugPutc {
    const SYSCALL_NO: usize = 1;
    type CallArgs = DebugPutcArgs;
    type Return = SyscallResult<NoValue>;
}
