//! Definitions for the `identify` syscall.

use crate::{
    back_to_enum, CAddr, RawSyscallArgs, SyscallBinding, SyscallResult, SyscallReturnData,
};
use core::convert::Infallible;

back_to_enum! {
    #[derive(Debug, PartialEq, Eq)]
    #[repr(usize)]
    pub enum CapabilityVariant {
        Uninit = 0,
        Memory = 1,
        CSpace = 2,
        VSpace = 3,
        Task = 4,
        Page = 5,
        IrqControl = 6,
        Irq = 7,
        Notification = 8,
        Devmem = 9,
        AsidControl = 10,
        Endpoint = 11,
    }
}

impl Into<usize> for CapabilityVariant {
    fn into(self) -> usize {
        self as usize
    }
}

pub struct Identify;

#[derive(Debug, Eq, PartialEq)]
pub struct IdentifyArgs {
    pub caddr: CAddr,
}

impl SyscallBinding for Identify {
    const SYSCALL_NO: usize = 3;
    type CallArgs = IdentifyArgs;
    type Return = SyscallResult<CapabilityVariant>;
}

impl From<IdentifyArgs> for RawSyscallArgs {
    fn from(args: IdentifyArgs) -> Self {
        [args.caddr.into(), 0, 0, 0, 0, 0, 0]
    }
}

impl TryFrom<RawSyscallArgs> for IdentifyArgs {
    type Error = Infallible;

    fn try_from(args: RawSyscallArgs) -> Result<Self, Self::Error> {
        Ok(Self {
            caddr: args[0].into(),
        })
    }
}

impl Into<SyscallReturnData> for CapabilityVariant {
    fn into(self) -> SyscallReturnData {
        [self as usize, 0, 0, 0, 0, 0, 0]
    }
}

impl TryFrom<SyscallReturnData> for CapabilityVariant {
    type Error = ();

    fn try_from(value: SyscallReturnData) -> Result<Self, Self::Error> {
        Self::try_from(value[0])
    }
}
