//! Kernel provided capabilities
#![deprecated(since = "0.5.0", note = "please use crate `derivation_tree` instead")]

mod capability;
pub mod cspace;
pub mod memory;
pub mod task;
pub mod vspace;

pub use capability::CapHolder;
