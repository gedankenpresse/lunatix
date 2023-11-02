use crate::caps::{CSpace, Capability, SyscallError};
use syscall_abi::CAddr;

pub(crate) unsafe fn lookup_cap(
    cspace: &CSpace,
    caddr: CAddr,
    expected_tag: crate::caps::Tag,
) -> Result<&'static Capability, SyscallError> {
    let cap_ptr = cspace
        .resolve_caddr(caddr)
        .ok_or(SyscallError::InvalidCap)?;
    // TODO Use a cursor to safely access the capability
    let cap = cap_ptr.as_ref().unwrap();
    if *cap.get_tag() != expected_tag {
        return Err(SyscallError::InvalidCap);
    }
    Ok(cap)
}

pub(crate) unsafe fn lookup_cap_mut(
    cspace: &CSpace,
    caddr: CAddr,
    expected_tag: crate::caps::Tag,
) -> Result<&'static mut Capability, SyscallError> {
    let cap_ptr = cspace
        .resolve_caddr(caddr)
        .ok_or(SyscallError::InvalidCap)?;
    // TODO Use a cursor to safely access the capability
    let cap = cap_ptr.as_mut().unwrap();
    if *cap.get_tag() != expected_tag {
        return Err(SyscallError::InvalidCap);
    }
    Ok(cap)
}
