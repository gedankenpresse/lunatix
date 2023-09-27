//! Definitions for the `derive_from_mem` syscall.

use crate::identify::CapabilityVariant;
use crate::{CAddr, NoValue, RawSyscallArgs, SyscallBinding, SyscallResult};

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

impl SyscallBinding for DeriveFromMem {
    const SYSCALL_NO: usize = 7;
    type CallArgs = DeriveFromMemArgs;
    type Return = SyscallResult<NoValue>;
}

impl From<DeriveFromMemArgs> for RawSyscallArgs {
    fn from(value: DeriveFromMemArgs) -> Self {
        [
            value.src_mem,
            value.target_slot,
            value.target_cap as usize,
            value.size.unwrap_or(0),
            0,
            0,
            0,
        ]
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
            },
        }
    }
}
