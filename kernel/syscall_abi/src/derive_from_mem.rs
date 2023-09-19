//! Definitions for the `derive_from_mem` syscall.

use crate::{CAddr, RawSyscallArgs, RawSyscallReturn, SyscallBinding};
use crate::generic_return::GenericReturn;
use crate::identify::CapabilityVariant;

pub struct DeriveFromMem;

#[derive(Debug, Eq, PartialEq)]
pub struct DeriveFromMemArgs {
    /// The CAddr of the memory capability from which another capability is to be derived.
    pub src_mem: CAddr,
    /// The CAddr of an empty slot into which the derived capability should be placed.
    pub target_slot: CAddr,
    /// Which capability should be derived.
    pub target_cap: CapabilityVariant,
    /// Size argument to the derivation (if applicable)
    pub size: Option<usize>,
}

#[derive(Debug, Eq, PartialEq)]
#[repr(usize)]
pub enum DeriveFromMemReturn {
    Success = 0,
    InvalidMemCAddr = 1,
    InvalidTargetCAddr = 2,
    OutOfMemory = 3,
    CannotBeDerived = 4,
    UnsupportedSyscall = usize::MAX,
}

impl SyscallBinding for DeriveFromMem {
    const SYSCALL_NO: usize = 7;
    type CallArgs = DeriveFromMemArgs;
    type Return = DeriveFromMemReturn;
}

impl From<DeriveFromMemArgs> for RawSyscallArgs {
    fn from(value: DeriveFromMemArgs) -> Self {
        [value.src_mem, value.target_slot, value.target_cap as usize, value.size.unwrap_or(0), 0, 0, 0]
    }
}

impl From<RawSyscallArgs> for DeriveFromMemArgs {
    fn from(value: RawSyscallArgs) -> Self {
        Self {
            src_mem: value[0],
            target_slot: value[1],
            target_cap: value[2].try_into().unwrap(),
            size: match value[3] {
                0 => None,
                v => Some(v),
            }
        }
    }
}

impl From<DeriveFromMemReturn> for RawSyscallReturn {
    fn from(value: DeriveFromMemReturn) -> Self {
        match value {
            DeriveFromMemReturn::Success => [0, 0],
            DeriveFromMemReturn::InvalidMemCAddr => [1, 0],
            DeriveFromMemReturn::InvalidTargetCAddr => [2, 0],
            DeriveFromMemReturn::OutOfMemory => [3, 0],
            DeriveFromMemReturn::CannotBeDerived => [4, 0],
            DeriveFromMemReturn::UnsupportedSyscall => [usize::MAX, 0],
        }
    }
}

impl From<RawSyscallReturn> for DeriveFromMemReturn {
    fn from(value: RawSyscallReturn) -> Self {
        match value[0] {
            0 => Self::Success,
            1 => Self::InvalidMemCAddr,
            2 => Self::InvalidTargetCAddr,
            3 => Self::OutOfMemory,
            4 => Self::CannotBeDerived,
            usize::MAX => Self::UnsupportedSyscall,
            _ => panic!("unknown syscall return (this should be handled better)")
        }
    }
}

impl From<DeriveFromMemReturn> for GenericReturn {
    fn from(value: DeriveFromMemReturn) -> Self {
        match value {
            DeriveFromMemReturn::Success => Self::Success,
            DeriveFromMemReturn::UnsupportedSyscall => Self::UnsupportedSyscall,
            _ => Self::Error,
        }
    }
}


