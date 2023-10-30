//! Definitions for the `send` syscall.
//!
//! `send` is a generic remote procedure call that does not immediately return a result.
//! It is implemented for performing actions on some builtin kernel objects but is also used for
//! inter process communication.

use crate::ipc_tag::IpcTag;
use crate::{CAddr, NoValue, RawSyscallArgs, SyscallBinding, SyscallResult};
use core::fmt::Debug;
use core::mem;

pub const NUM_DATA_REGS: usize = 5;

pub struct Send;

#[derive(Debug, Eq, PartialEq)]
pub struct SendArgs {
    /// The object on which a send is performed.
    pub target: CAddr,

    /// A tag containing the metadata of this send
    pub tag: IpcTag,

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
        let slice = &self.raw_args[..self.tag.ncaps() as usize];
        unsafe { mem::transmute::<&[usize], &[CAddr]>(slice) }
    }

    /// Return the inline data that is included as argument to this send call
    pub fn data_args(&self) -> &[usize] {
        &self.raw_args[self.tag.ncaps() as usize..(self.tag.ncaps() + self.tag.nparams()) as usize]
    }

    /// The label of this IPC operation.
    ///
    /// Often used to communicate what should be done and can be though of serving a similar purpose to a syscall number.
    pub fn label(&self) -> usize {
        self.tag.label()
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
            target: value[0].into(),
            tag: IpcTag::from_raw(value[1]),
            raw_args: [value[2], value[3], value[4], value[5], value[6]],
        }
    }
}

impl From<SendArgs> for RawSyscallArgs {
    fn from(value: SendArgs) -> Self {
        [
            value.target.into(),
            value.tag.as_raw(),
            value.raw_args[0],
            value.raw_args[1],
            value.raw_args[2],
            value.raw_args[3],
            value.raw_args[4],
        ]
    }
}
