#![no_std]

#[cfg(feature = "std")]
extern crate std;

mod arena_allocator;
pub mod bump_allocator;
mod custom_box;
mod traits;

pub use arena_allocator::Arena;
pub use custom_box::Box;
use thiserror_no_std::Error;
pub use traits::{AllocError, AllocInit, Allocator};
