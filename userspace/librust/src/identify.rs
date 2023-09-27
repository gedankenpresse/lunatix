use crate::syscalls::syscall;
use syscall_abi::{
    identify::{CapabilityVariant, Identify, IdentifyArgs},
    SyscallResult,
};

pub fn identify(caddr: usize) -> SyscallResult<CapabilityVariant> {
    syscall::<Identify>(IdentifyArgs { caddr })
}
