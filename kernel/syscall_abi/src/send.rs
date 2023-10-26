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

    /// Raw arguments to this RPC.
    ///
    /// These should not be interpreted directly because they may contain sent capabilities as well as inline data.
    /// Instead either [`cap_args()`](SendArgs::cap_args) or [`data_args()`](SendArgs::data_args) should be called
    /// to retrieve the expected types of arguments.
    pub raw_args: [usize; NUM_DATA_REGS],
}

impl SendArgs {
    /// Return the capabilities that are included as arguments to this send call
    pub fn cap_args(&self) -> &[CAddr] {
        &self.raw_args[..self.num_caps as usize]
    }

    /// Return the inline data that is included as argument to this send call
    pub fn data_args(&self) -> &[CAddr] {
        &self.raw_args[self.num_caps as usize..]
    }
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
            raw_args: [value[2], value[3], value[4], value[5], value[6]],
        }
    }
}

impl From<SendArgs> for RawSyscallArgs {
    fn from(value: SendArgs) -> Self {
        [
            value.target,
            ((value.op as usize) << 16) | (value.num_caps as usize),
            value.raw_args[0],
            value.raw_args[1],
            value.raw_args[2],
            value.raw_args[3],
            value.raw_args[4],
        ]
    }
}
