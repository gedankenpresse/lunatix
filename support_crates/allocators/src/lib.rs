#![no_std]

#[cfg(feature = "std")]
extern crate std;

mod arena_allocator;
mod boundary_tag_alloc;
pub mod bump_allocator;
mod custom_box;
mod stack_allocator;
mod traits;

pub use arena_allocator::Arena;
pub use arena_allocator::ArenaAlloc;
pub use boundary_tag_alloc::BoundaryTagAllocator;
pub use custom_box::Box;
pub use traits::{AllocError, AllocInit, Allocator, MutGlobalAlloc};
