use core::alloc::Layout;
use core::alloc::LayoutError;
use thiserror_no_std::Error;

/// The error returned when an allocation fails
#[derive(Debug, Error, Eq, PartialEq)]
pub enum AllocError {
    #[error("the allocator has insufficient free memory to allocate the requested amount")]
    InsufficientMemory,
    #[error("the requested layout could not be fulfilled")]
    LayoutError(#[from] LayoutError),
}

/// A desired initial state for allocated memory
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

/// An implementation of `Allocator` can allocate and deallocate arbitrary blocks of data.
pub trait Allocator<'mem> {
    /// Attempt to allocate a block of memory.
    ///
    /// On success, return a slice of memory meeting the size and alignment requirements of `layout`.
    ///
    /// The returned block may or may not have its content initialized based on the value of `init`.
    ///
    /// # Panics
    /// Allocator implementations are not required to support zero-sized allocations and may panic when one is
    /// requested.
    fn allocate(&self, layout: Layout, init: AllocInit) -> Result<&'mem mut [u8], AllocError>;

    /// Deallocate the given data.
    ///
    /// # Panics
    /// This function may panic if the given `data_ptr` does not lie within the bounds of the allocators backing memory.
    ///
    /// # Safety
    /// The given data must be *currently allocated* from this allocator.
    ///
    /// This means that:
    /// - it was previously returned by [`allocate`](BumpAllocator::allocate)
    /// - it has not yet been deallocated
    unsafe fn deallocate(&self, data_ptr: *mut u8, layout: Layout);
}
