//! Definitions for the `yield` syscall

use crate::{NoValue, SyscallBinding, SyscallResult};

pub struct Yield;

pub type YieldReturn = SyscallResult<NoValue>;

impl SyscallBinding for Yield {
    const SYSCALL_NO: usize = 12;
    type CallArgs = NoValue;
    type Return = SyscallResult<NoValue>;
}
