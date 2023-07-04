#![no_std]

#[cfg(feature = "std")]
extern crate std;

mod arena_allocator;
mod bump_allocator;
mod bump_box;

pub use arena_allocator::Arena;
pub use bump_allocator::{AllocError, AllocInit, BumpAllocator};
pub use bump_box::BumpBox;
