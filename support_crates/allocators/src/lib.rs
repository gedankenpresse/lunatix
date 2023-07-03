#![no_std]

#[cfg(feature = "std")]
extern crate std;

mod arena_allocator;
mod bump_allocator;

pub use arena_allocator::Arena;
pub use bump_allocator::BumpAllocator;
