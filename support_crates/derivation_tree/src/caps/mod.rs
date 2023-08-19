//! A number of builtin capabilities that are essential for using the DerivationTree
mod cspace;
mod iface;
mod memory;
mod uninit;
mod uninit_slot;

pub use cspace::CSpace;
pub use iface::{CapabilityIface, GetCapIface};
pub use memory::Memory;
pub use uninit::Uninit;
pub use uninit_slot::UninitSlot;

#[cfg(test)]
pub mod test_union;
