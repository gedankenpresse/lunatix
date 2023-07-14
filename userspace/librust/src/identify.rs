use crate::syscalls::{syscall, self};

/// a capability variant
#[derive(Debug)]
#[repr(usize)]
pub enum Variant {
    Uninit = 0,
    Memory = 1,
    CSpace = 2,
    VSpace = 3,
    Task = 4,
}


impl TryFrom<usize> for Variant {
    type Error = crate::Error;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Uninit),
            1 => Ok(Self::Memory),
            2 => Ok(Self::CSpace),
            3 => Ok(Self::VSpace),
            4 => Ok(Self::Task),
            _ => Err(crate::Error::InvalidReturn),
        }
    }
}

pub fn identify(cap: usize) -> Result<Variant, crate::Error> {
    let v = syscall(syscalls::SYS_IDENTIFY, cap, 0, 0, 0, 0, 0, 0)?;
    v.try_into()
}