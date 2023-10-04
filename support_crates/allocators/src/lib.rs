#![no_std]

#[cfg(feature = "std")]
extern crate std;

mod arena_allocator;
pub mod boundary_tag_alloc;
pub mod bump_allocator;
mod custom_box;
mod stack_allocator;
mod traits;

pub use arena_allocator::Arena;
pub use arena_allocator::ArenaAlloc;
pub use custom_box::Box;
pub use traits::{AllocError, AllocInit, Allocator, MutGlobalAlloc};
