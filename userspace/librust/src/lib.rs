#![no_std]

pub mod identify;
pub(crate) mod syscalls;
// pub(crate) mod ipc;
// pub mod memory;
mod assign_ipc_buffer;
mod derive_from_mem;
mod map_page;
pub mod print;
mod task_assign_cspace;
mod task_assign_vspace;

pub use assign_ipc_buffer::assign_ipc_buffer;
pub use derive_from_mem::derive_from_mem;
pub use identify::identify;
pub use map_page::map_page;
pub use print::print;
pub use print::put_c;
pub use task_assign_cspace::task_assign_cspace;
pub use task_assign_vspace::task_assign_vspace;
// pub use ipc::IpcResult;
// pub use memory::allocate;

pub use syscall_abi;

// #[repr(usize)]
// #[derive(Debug, PartialEq, Eq)]
// pub enum Error {
//     InvalidCAddr = 1,
//     NoMem = 2,
//     OccupiedSlot = 3,
//     InvalidCap = 4,
//     InvalidOp = 5,
//     InvalidArg = 6,
//     AliasingCSlot = 7,
//     InvalidReturn = 8,
// }
//
// impl From<usize> for Error {
//     fn from(value: usize) -> Self {
//         match value {
//             0 => Error::InvalidReturn,
//             1 => Error::InvalidCAddr,
//             2 => Error::NoMem,
//             3 => Error::OccupiedSlot,
//             4 => Error::InvalidCap,
//             5 => Error::InvalidOp,
//             6 => Error::InvalidArg,
//             7 => Error::AliasingCSlot,
//             _ => Error::InvalidReturn,
//         }
//     }
// }
//
// /// a capability variant
// #[derive(Debug, PartialEq, Eq)]
// #[repr(usize)]
// pub enum Variant {
//     Uninit = 0,
//     Memory = 1,
//     CSpace = 2,
//     VSpace = 3,
//     Task = 4,
//     Page = 5,
// }
//
// impl TryFrom<usize> for Variant {
//     type Error = crate::Error;
//
//     fn try_from(value: usize) -> Result<Self, Self::Error> {
//         match value {
//             0 => Ok(Self::Uninit),
//             1 => Ok(Self::Memory),
//             2 => Ok(Self::CSpace),
//             3 => Ok(Self::VSpace),
//             4 => Ok(Self::Task),
//             5 => Ok(Self::Page),
//             _ => Err(crate::Error::InvalidReturn),
//         }
//     }
// }
