//! Definitions for the `identify` syscall.

use crate::generic_return::GenericReturn;
use crate::{RawSyscallArgs, RawSyscallReturn, SyscallBinding};
use core::convert::Infallible;

#[derive(Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum CapabilityVariant {
    Uninit = 0,
    Memory = 1,
    CSpace = 2,
    VSpace = 3,
    Task = 4,
    Page = 5,
}

pub struct Identify;

#[derive(Debug, Eq, PartialEq)]
pub struct IdentifyArgs {
    pub caddr: usize,
}

#[derive(Debug, Eq, PartialEq)]
#[repr(C, usize)]
pub enum IdentifyReturn {
    Success(CapabilityVariant) = 0,
    InvalidCAddr = 1,
    UnsupportedSyscall = usize::MAX,
}

impl SyscallBinding for Identify {
    const SYSCALL_NO: usize = 3;
    type CallArgs = IdentifyArgs;
    type Return = IdentifyReturn;
}

impl From<IdentifyArgs> for RawSyscallArgs {
    fn from(args: IdentifyArgs) -> Self {
        [args.caddr, 0, 0, 0, 0, 0, 0]
    }
}

impl TryFrom<RawSyscallArgs> for IdentifyArgs {
    type Error = Infallible;

    fn try_from(args: RawSyscallArgs) -> Result<Self, Self::Error> {
        Ok(Self { caddr: args[0] })
    }
}

impl Into<RawSyscallReturn> for IdentifyReturn {
    fn into(self) -> RawSyscallReturn {
        // SAFETY: Because `Self` is marked `repr(usize)`, its layout is a `repr(C)` `union`
        // between `repr(C)` structs, each of which has the `usize` discriminant as its first
        // field, so we can read the discriminant without offsetting the pointer.
        let reg0 = unsafe { *<*const _>::from(&self).cast::<usize>() };

        let reg1 = match self {
            IdentifyReturn::Success(variant) => variant as usize,
            IdentifyReturn::InvalidCAddr => 1,
            IdentifyReturn::UnsupportedSyscall => usize::MAX,
        };

        [reg0, reg1]
    }
}

#[derive(Debug, Eq, PartialEq)]
#[repr(usize)]
pub enum ReturnValueDecodingError {
    UnknownReturnCode = 0,
    UnknownCapabilityVariant = 1,
}

impl TryFrom<RawSyscallReturn> for IdentifyReturn {
    type Error = ReturnValueDecodingError;

    fn try_from(raw_return: RawSyscallReturn) -> Result<Self, Self::Error> {
        let [reg0, reg1] = raw_return;

        Ok(match reg0 {
            0 => IdentifyReturn::Success(match reg1 {
                0 => CapabilityVariant::Uninit,
                1 => CapabilityVariant::Memory,
                2 => CapabilityVariant::CSpace,
                3 => CapabilityVariant::VSpace,
                4 => CapabilityVariant::Task,
                5 => CapabilityVariant::Page,
                _ => return Err(ReturnValueDecodingError::UnknownCapabilityVariant),
            }),
            1 => IdentifyReturn::InvalidCAddr,
            usize::MAX => IdentifyReturn::UnsupportedSyscall,
            _ => return Err(ReturnValueDecodingError::UnknownReturnCode),
        })
    }
}

impl Into<GenericReturn> for IdentifyReturn {
    fn into(self) -> GenericReturn {
        match self {
            IdentifyReturn::Success(_) => GenericReturn::Success,
            IdentifyReturn::InvalidCAddr => GenericReturn::Error,
            IdentifyReturn::UnsupportedSyscall => GenericReturn::UnsupportedSyscall,
        }
    }
}

impl TryFrom<usize> for CapabilityVariant {
    type Error = ();

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Uninit),
            1 => Ok(Self::Memory),
            2 => Ok(Self::CSpace),
            3 => Ok(Self::VSpace),
            4 => Ok(Self::Task),
            5 => Ok(Self::Page),
            _ => Err(()),
        }
    }
}
