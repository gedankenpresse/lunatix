use crate::caps::{CSpace, Capability, Error};

pub(crate) unsafe fn lookup_cap(
    cspace: &CSpace,
    caddr: usize,
    expected_tag: crate::caps::Tag,
) -> Result<&'static Capability, Error> {
    let cap_ptr = cspace.lookup_raw(caddr).ok_or(Error::InvalidCap)?;
    // TODO Use a cursor to safely access the capability
    let cap = cap_ptr.as_ref().unwrap();
    if *cap.get_tag() != expected_tag {
        return Err(Error::InvalidCap);
    }
    Ok(cap)
}

pub(crate) unsafe fn lookup_cap_mut(
    cspace: &CSpace,
    caddr: usize,
    expected_tag: crate::caps::Tag,
) -> Result<&'static mut Capability, Error> {
    let cap_ptr = cspace.lookup_raw(caddr).ok_or(Error::InvalidCap)?;
    // TODO Use a cursor to safely access the capability
    let cap = cap_ptr.as_mut().unwrap();
    if *cap.get_tag() != expected_tag {
        return Err(Error::InvalidCap);
    }
    Ok(cap)
}