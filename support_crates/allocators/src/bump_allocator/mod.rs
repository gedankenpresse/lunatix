//! Bump-Allocator
//!
//! See the [`BumpAllocator`] trait for a detailed description of bump allocators.

mod backward_alloc;
mod bump_alloc_trait;
mod bump_box;
mod forward_alloc;

pub use backward_alloc::BackwardBumpingAllocator;
pub use bump_alloc_trait::BumpAllocator;
pub use bump_box::BumpBox;
pub use forward_alloc::ForwardBumpingAllocator;
