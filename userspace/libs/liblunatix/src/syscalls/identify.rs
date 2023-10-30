use crate::syscalls::syscall;
use syscall_abi::{
    identify::{CapabilityVariant, Identify, IdentifyArgs},
    CAddr, SyscallResult,
};

pub fn identify(caddr: CAddr) -> SyscallResult<CapabilityVariant> {
    syscall::<Identify>(IdentifyArgs {
        caddr: caddr.into(),
    })
}
