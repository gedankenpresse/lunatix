//! Definitions for the `wait_on` syscall.

use crate::{CAddr, RawSyscallArgs, SyscallBinding, SyscallResult};

pub struct WaitOn;

#[derive(Debug, Eq, PartialEq)]
pub struct WaitOnArgs {
    /// The notification capability to wait on
    pub notification: CAddr,
}

impl SyscallBinding for WaitOn {
    const SYSCALL_NO: usize = 14;
    type CallArgs = WaitOnArgs;
    type Return = SyscallResult<usize>;
}

impl From<RawSyscallArgs> for WaitOnArgs {
    fn from(value: RawSyscallArgs) -> Self {
        Self {
            notification: value[0],
        }
    }
}

impl From<WaitOnArgs> for RawSyscallArgs {
    fn from(value: WaitOnArgs) -> Self {
        [value.notification, 0, 0, 0, 0, 0, 0]
    }
}
