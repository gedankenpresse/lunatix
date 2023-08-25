//! Definitions for the `debug_putc` syscall

use crate::generic_return::GenericReturn;
use crate::{RawSyscallArgs, SyscallBinding};
use core::convert::Infallible;

pub struct DebugPutc;

pub struct DebugPutcArgs(pub char);

pub type DebugPutcReturn = GenericReturn;

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
    type Return = DebugPutcReturn;
}
