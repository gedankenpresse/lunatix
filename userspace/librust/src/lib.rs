#![no_std]

pub(crate) mod syscalls;
pub mod print;
pub mod identify;

pub use print::print;
pub use print::put_c;
pub use identify::identify;

#[repr(usize)]
#[derive(Debug)]
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