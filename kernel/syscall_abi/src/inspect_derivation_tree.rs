//! Definitions for the `inspect_derivation_tree` syscall

use crate::inspect_derivation_tree::ipc_types::CapabilityList;
use crate::{IpcReturnBinding, NoValue, RawSyscallArgs, SyscallBinding, SyscallResult};

pub struct InspectDerivationTree {}

pub struct InspectDerivationTreeArgs {}

impl SyscallBinding for InspectDerivationTree {
    const SYSCALL_NO: usize = 0;
    type CallArgs = InspectDerivationTreeArgs;
    type Return = SyscallResult<NoValue>;
}

impl IpcReturnBinding for InspectDerivationTree {
    type IpcReturn = CapabilityList;
}

impl Into<RawSyscallArgs> for InspectDerivationTreeArgs {
    fn into(self) -> RawSyscallArgs {
        [0; 7]
    }
}

impl From<RawSyscallArgs> for InspectDerivationTreeArgs {
    fn from(_: RawSyscallArgs) -> Self {
        Self {}
    }
}

pub mod ipc_types {
    #[derive(Debug)]
    #[repr(C)]
    pub struct CapabilityList {
        len: usize,
        start: *const CapabilityDescription,
    }

    #[derive(Debug)]
    #[repr(usize)]
    pub enum CapabilityDescription {
        Uninit = 0,
        Memory(MemoryDescription) = 1,
        CSpace(CSpaceDescription) = 2,
        VSpace(VSpaceDescription) = 3,
        Task(TaskDescription) = 4,
        Page(PageDescription) = 5,
    }

    #[derive(Debug)]
    #[repr(C)]
    pub struct MemoryDescription {
        bytes_used: usize,
    }

    #[derive(Debug)]
    #[repr(C)]
    pub struct CSpaceDescription {
        slots: CapabilityList,
    }

    #[derive(Debug)]
    #[repr(C)]
    pub struct VSpaceDescription {}

    #[derive(Debug)]
    #[repr(C)]
    pub struct TaskDescription {}

    #[derive(Debug)]
    #[repr(C)]
    pub struct PageDescription {}
}
