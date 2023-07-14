#![no_std]

pub(crate) mod syscalls;
pub(crate) mod ipc;
pub mod print;
pub mod identify;
pub mod memory;


pub use print::print;
pub use print::put_c;
pub use identify::identify;
pub use memory::allocate;
pub use ipc::IpcResult;

#[repr(usize)]
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    InvalidCAddr = 1,
    NoMem = 2,
    OccupiedSlot = 3,
    InvalidCap = 4,
    InvalidOp = 5,
    InvalidArg = 6,
    AliasingCSlot = 7,
    InvalidReturn = 8,
}

impl From<usize> for Error {
    fn from(value: usize) -> Self {
        match value {
            0 => Error::InvalidReturn,
            1 => Error::InvalidCAddr,
            2 => Error::NoMem,
            3 => Error::OccupiedSlot,
            4 => Error::InvalidCap,
            5 => Error::InvalidOp,
            6 => Error::InvalidArg,
            7 => Error::AliasingCSlot,
            _ => Error::InvalidReturn,
        }
    }
}

/// a capability variant
#[derive(Debug, PartialEq, Eq)]
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