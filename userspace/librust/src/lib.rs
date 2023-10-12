#![no_std]

pub mod copy;
pub mod destroy;
pub mod identify;
pub(crate) mod syscalls;
// pub(crate) mod ipc;
mod assign_ipc_buffer;

mod asid;
mod devmem;
mod irq;
mod irq_control;
mod mem;
mod page;
mod task;

pub mod print;
mod system_reset;
mod wait_on;
mod r#yield;
mod yield_to;

pub use asid::asid_assign;
pub use assign_ipc_buffer::assign_ipc_buffer;
pub use copy::copy;
pub use destroy::destroy;
pub use devmem::devmem_map;
pub use identify::identify;
pub use irq::irq_complete;
pub use irq_control::irq_control_claim;
pub use mem::derive;
pub use page::map_page;
pub use print::print;
pub use print::put_c;
pub use r#yield::r#yield;
pub use system_reset::system_reset;
pub use task::task_assign_control_registers;
pub use task::task_assign_cspace;
pub use task::task_assign_vspace;
pub use wait_on::wait_on;
pub use yield_to::yield_to;
// pub use ipc::IpcResult;
// pub use memory::allocate;

pub use syscall_abi;

pub mod prelude {
    pub use crate::print;
    pub use crate::println;
    pub use crate::syscall_abi::CAddr;
    pub use crate::syscall_abi::Error;
    pub use crate::syscall_abi::NoValue;
}

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
