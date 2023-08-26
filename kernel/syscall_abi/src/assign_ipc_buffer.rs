//! Definitions for the `assign_ipc_buffer` syscall.

use crate::generic_return::GenericReturn;
use crate::{CAddr, RawSyscallArgs, SyscallBinding};
use core::convert::Infallible;

pub struct AssignIpcBuffer;

#[derive(Debug, Eq, PartialEq)]
#[repr(C)]
pub struct AssignIpcBufferArgs {
    /// The CAddr of a memory capability that should be used as IPC buffer
    pub page: CAddr,
}

pub type AssignIpcBufferReturn = GenericReturn;

impl SyscallBinding for AssignIpcBuffer {
    const SYSCALL_NO: usize = 6;
    type CallArgs = AssignIpcBufferArgs;
    type Return = AssignIpcBufferReturn;
}

impl Into<RawSyscallArgs> for AssignIpcBufferArgs {
    fn into(self) -> RawSyscallArgs {
        [self.page, 0, 0, 0, 0, 0, 0]
    }
}

impl TryFrom<RawSyscallArgs> for AssignIpcBufferArgs {
    type Error = Infallible;

    fn try_from(args: RawSyscallArgs) -> Result<Self, Self::Error> {
        Ok(Self { page: args[0] })
    }
}
