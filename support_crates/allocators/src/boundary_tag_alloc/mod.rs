//! Implementation of the boundary tagged allocator.
//!
//! The main type is the [`BoundaryTagAllocator`] which implements the `Allocator` trait and uses a specific tag
//! type ([`TagsU8`], [`TagsU16`] or [`TagsUsize`]) for bookkeeping.
//!
//! The different tag types model a trade-off between supporting larger amounts of backing memory and needing more
//! space.
//! For example, the `TagsUsize` type supports up to ~18.5 exabytes but uses up 17 bytes for bookkeeping per
//! allocation even if the allocation is for very small data.
//! On the other hand `TagsU8` supports only 255 bytes of backing memory but only uses 3 bytes per allocation for
//! bookkeeping.
//! The details are encoded in the [`TagsBinding`] trait implementation for each tag type.
//!
//! # Example
//!
//! ## Perform a raw allocation and deallocation
//!
//! This example shows how a `BoundaryTagAllocator` using `u16` based tags is created on the stack.
//! These tags support up to `65_535` bytes of backing memory.
//! After creation, the allocator is used to do an allocation for one `usize` followed by the deallocation of that
//! allocation.
//!
//! ```rust
//! # use std::alloc::Layout;
//! # use allocators::{Allocator, AllocInit, stack_alloc};
//! # use allocators::boundary_tag_alloc::{BoundaryTagAllocator, TagsU16};
//! #
//! // create an allocator called `allocator` with u16 based tags
//! stack_alloc!(allocator, 1024, BoundaryTagAllocator<TagsU16>);
//!
//! let allocation = allocator.allocate(Layout::new::<usize>(), AllocInit::Zeroed).unwrap();
//! unsafe { allocator.deallocate(allocation.as_mut_ptr(), Layout::new::<usize>()) };
//! ```
//!
//! ## Use the allocator with a box
//! ```rust
//! # use std::alloc::Layout;
//! # use allocators::{Allocator, AllocInit, stack_alloc, Box};
//! # use allocators::boundary_tag_alloc::{BoundaryTagAllocator, TagsU8};
//! #
//! stack_alloc!(allocator, 64, BoundaryTagAllocator<TagsU8>);
//! let b = Box::new(0x55, &allocator).unwrap();
//! assert_eq!(*b, 0x55);
//! ```
//!
mod allocator;
mod tags;

#[cfg(test)]
mod tests;

pub use allocator::*;
pub use tags::*;
