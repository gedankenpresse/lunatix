use crate::syscalls::send;
use syscall_abi::identify::CapabilityVariant;
use syscall_abi::{CAddr, FromRawSysResponse, NoValue, SyscallResult};

pub fn derive(
    mem: CAddr,
    target: CAddr,
    variant: CapabilityVariant,
    size: Option<usize>,
) -> SyscallResult<NoValue> {
    const DERIVE: usize = 1;
    send(
        mem,
        DERIVE,
        &[target],
        &[variant.into(), size.unwrap_or(0), 0, 0],
    )
}
