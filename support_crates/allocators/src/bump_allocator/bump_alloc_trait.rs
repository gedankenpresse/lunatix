use crate::Allocator;
use core::ptr;

/// A trait defining the common behavior between different bump allocators.
///
/// Generally a bump allocator is implemented using a backing memory buffer in addition to a marker tracking how many
/// bytes are already allocated.
///
/// ```text
///   ┌────────────────── backing memory ────────────────────┐
///   │                                                      │
/// [0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0]
///                  ^
///                  │
///               marker
/// ```
///
/// When an allocation is performed, the marker is *bumped* forward to track that more space has been allocated.
///
/// Deallocations on the other hand don't usually make the freed memory available again because the allocator lacks
/// the capability to track free holes in the backing memory.
/// However, when **all** allocations are returned to the allocator, the marker is reset to `0` thus making the whole
/// memory available for allocations again.
pub trait BumpAllocator<'mem>: Sized + Allocator<'mem> {
    /// Create a new bump allocator that allocates from the given backing memory region.
    fn new(backing_mem: &'mem mut [u8]) -> Self;

    /// Create a new allocator that allocates from the memory region between `start` and `end`
    ///
    /// # Safety
    /// The entire memory area must be accessible and otherwise completely unused.
    unsafe fn new_raw(start: *mut u8, end: *mut u8) -> Self {
        assert!(start <= end);
        Self::new(&mut *ptr::slice_from_raw_parts_mut(
            start,
            end as usize - start as usize,
        ))
    }

    /// Steal the remaining free memory from the allocator.
    ///
    /// This makes the stolen memory unavailable to the allocator so that no further regions are allocated from it.
    fn steal_remaining_mem(&self) -> &'mem mut [u8];
}
