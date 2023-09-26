//! Definitions for the `yield` syscall

use crate::generic_return::GenericReturn;
use crate::{RawSyscallArgs, SyscallBinding};

pub struct Yield;

#[derive(Debug, Eq, PartialEq)]
pub struct YieldArgs {}

pub type YieldReturn = GenericReturn;

impl SyscallBinding for Yield {
    const SYSCALL_NO: usize = 12;
    type CallArgs = YieldArgs;
    type Return = YieldReturn;
}

impl From<YieldArgs> for RawSyscallArgs {
    fn from(_value: YieldArgs) -> Self {
        [0, 0, 0, 0, 0, 0, 0]
    }
}

impl From<RawSyscallArgs> for YieldArgs {
    fn from(_value: RawSyscallArgs) -> Self {
        Self {}
    }
}
