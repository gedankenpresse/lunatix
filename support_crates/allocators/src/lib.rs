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

/// The error returned when an allocation fails
#[deprecated]
#[derive(Debug, Error)]
pub enum AllocFailed {
    #[error("the allocator has insufficient free memory to allocate the requested amount")]
    InsufficientMemory,
}

/// A desired initial state for allocated memory
#[deprecated]
#[derive(Default, Debug, Eq, PartialEq)]
pub enum AllocInit {
    /// The memory is returned as-is from the allocator.
    /// It may potentially contain old data and treating it as valid is undefined behavior.
    Uninitialized,

    /// Memory is filled with zeros before being returned to the caller.
    #[default]
    Zeroed,

    /// Memory is filled with a repetition of the given byte before being returned to the caller.
    Data(u8),
}
