//! Definitions for the `send` syscall.
//!
//! `send` is a generic remote procedure call that does not immediately return a result.
//! It is implemented for performing actions on some builtin kernel objects but is also used for
//! inter process communication.

use crate::{CAddr, NoValue, RawSyscallArgs, SyscallBinding, SyscallResult};
use core::mem;

const NUM_DATA_REGS: usize = 5;

pub struct Send;

#[derive(Debug, Eq, PartialEq)]
pub struct SendArgs {
    /// The object on which a send is performed.
    pub target: CAddr,

    /// The operation that should be performed.
    pub op: u16,

    /// How many of the arguments are capabilities.
    ///
    /// This is necessary to encode because the kernel needs to handle sent capabilities differently from plain old
    /// data.
    /// The remainder of `args` is interpreted to be plain old data though.
    pub num_caps: u16,

    /// Arguments to this RPC
    pub args: [usize; NUM_DATA_REGS],
}

impl SyscallBinding for Send {
    const SYSCALL_NO: usize = 18;
    type CallArgs = SendArgs;
    type Return = SyscallResult<NoValue>;
}

impl From<RawSyscallArgs> for SendArgs {
    fn from(value: RawSyscallArgs) -> Self {
        Self {
            target: value[0],
            // take the first 16 bits of value[1]
            op: (value[1] >> 16) as u16,
            // mask out and take the last 16 bits of value[1]
            num_caps: (value[1] & (!0u16 as usize)) as u16,
            args: [value[2], value[3], value[4], value[5], value[6]],
        }
    }
}

impl From<SendArgs> for RawSyscallArgs {
    fn from(value: SendArgs) -> Self {
        [
            value.target,
            ((value.op as usize) << 16) | (value.num_caps as usize),
            value.args[0],
            value.args[1],
            value.args[2],
            value.args[3],
            value.args[4],
        ]
    }
}
