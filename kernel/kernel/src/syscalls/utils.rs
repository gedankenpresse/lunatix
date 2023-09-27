use syscall_abi::SysError;

use crate::caps::{CSpace, Capability};

pub(crate) unsafe fn lookup_cap(
    cspace: &CSpace,
    caddr: usize,
    expected_tag: crate::caps::Tag,
) -> Result<&'static Capability, SysError> {
    let cap_ptr = cspace.lookup_raw(caddr).ok_or(SysError::InvalidCaddr)?;
    // TODO Use a cursor to safely access the capability
    let cap = cap_ptr.as_ref().unwrap();
    if *cap.get_tag() != expected_tag {
        return Err(SysError::UnexpectedCap);
    }
    Ok(cap)
}

pub(crate) unsafe fn lookup_cap_mut(
    cspace: &CSpace,
    caddr: usize,
    expected_tag: crate::caps::Tag,
) -> Result<&'static mut Capability, SysError> {
    let cap_ptr = cspace.lookup_raw(caddr).ok_or(SysError::InvalidCaddr)?;
    // TODO Use a cursor to safely access the capability
    let cap = cap_ptr.as_mut().unwrap();
    if *cap.get_tag() != expected_tag {
        return Err(SysError::UnexpectedCap);
    }
    Ok(cap)
}
