//! Implementation of the boundary tagged allocator
//!
//! ## Wordings
//!
//! In the implementation, some words are used with specific meaning:
//! - **Chunk**:
//!     A part of the memory which the allocator manages.
//!     Each chunk consists of the parts `begin-tag, padding, content, padding, end-tag`.
//!     The padding parts are optional and only necessary in certain edge cases.
//! - **Content**:
//!     A part of the allocators memory that is handed out to to users.
//! - **Padding**:
//!     A part of the allocators memory that serves no use.
//!     It sits between a tag and handed out content in certain situations but is neither handed
//!     out to the user as content nor otherwise used by the allocator.
//! - **Tag**:
//!     A tag stores metadata about the chunk it is contained in and fulfills a bookkeeping role.
//!     They come in the two flavors `begin-tag` which lies at the beginning of a chunk and `end-tag` which is located
//!     at the end of one.
//!

mod allocator;

mod tags;
#[cfg(test)]
mod tests;
