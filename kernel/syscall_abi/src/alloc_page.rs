//! Definitions for the `alloc_page` syscall.

use crate::generic_return::{GenericReturn, UnidentifiableReturnCode};
use crate::{CAddr, RawSyscallArgs, RawSyscallReturn, SyscallBinding};
use core::convert::Infallible;

pub struct AllocPage;

#[derive(Debug, Eq, PartialEq)]
pub struct AllocPageArgs {
    /// The CAddr of the memory capability from which a page should be allocated.
    pub src_mem: CAddr,
    /// The CAddr of an empty slot into which the allocated page capability should be placed.
    pub target_slot: CAddr,
}

#[derive(Debug, Eq, PartialEq)]
#[repr(usize)]
pub enum AllocPageReturn {
    Success = 0,
    InvalidMemCAddr = 1,
    InvalidTargetCAddr = 2,
    OutOfMemory = 3,
    UnsupportedSyscall = usize::MAX,
}

impl SyscallBinding for AllocPage {
    const SYSCALL_NO: usize = 4;
    type CallArgs = AllocPageArgs;
    type Return = AllocPageReturn;
}

impl From<AllocPageArgs> for RawSyscallArgs {
    fn from(args: AllocPageArgs) -> Self {
        [args.src_mem, args.target_slot, 0, 0, 0, 0, 0]
    }
}

impl TryFrom<RawSyscallArgs> for AllocPageArgs {
    type Error = Infallible;

    fn try_from(args: RawSyscallArgs) -> Result<Self, Self::Error> {
        Ok(Self {
            src_mem: args[0],
            target_slot: args[1],
        })
    }
}

impl Into<RawSyscallReturn> for AllocPageReturn {
    fn into(self) -> RawSyscallReturn {
        [self as usize, 0]
    }
}

impl TryFrom<RawSyscallReturn> for AllocPageReturn {
    type Error = UnidentifiableReturnCode;

    fn try_from(value: RawSyscallReturn) -> Result<Self, Self::Error> {
        match &value[0] {
            0 => Ok(AllocPageReturn::Success),
            1 => Ok(AllocPageReturn::InvalidMemCAddr),
            2 => Ok(AllocPageReturn::InvalidTargetCAddr),
            3 => Ok(AllocPageReturn::OutOfMemory),
            _ => Err(UnidentifiableReturnCode),
        }
    }
}

impl Into<GenericReturn> for AllocPageReturn {
    fn into(self) -> GenericReturn {
        match self {
            AllocPageReturn::Success => GenericReturn::Success,
            AllocPageReturn::InvalidMemCAddr => GenericReturn::Error,
            AllocPageReturn::InvalidTargetCAddr => GenericReturn::Error,
            AllocPageReturn::OutOfMemory => GenericReturn::Error,
            AllocPageReturn::UnsupportedSyscall => GenericReturn::Error,
        }
    }
}
