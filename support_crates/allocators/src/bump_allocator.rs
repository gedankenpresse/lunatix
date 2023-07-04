use core::{mem, ptr};
use ksync::SpinLock;
use thiserror_no_std::Error;

#[derive(Debug, Error)]
pub enum AllocError {
    #[error("the allocator has insufficient free memory to allocate the requested amount")]
    InsufficientMemory,
}

/// A desired initial state for allocated memory
#[derive(Default, Debug, Eq, PartialEq)]
pub enum AllocInit {
    #[default]
    Uninitialized,
    Zeroed,
    Data(u8),
}

#[derive(Debug, Eq, PartialEq, Hash)]
pub struct AllocatorState<'mem> {
    backing_mem: &'mem mut [u8],
    bytes_allocated: usize,
    num_allocations: usize,
}

/// A simple allocator implementation which bumps a marker in the backing memory to denote what has already been allocated.
///
/// As a consequence of the simple allocator design, arbitrary size & alignment allocations are support but de-allocation
/// does immediately make the allocated memory available to be allocated again.
/// Only when *all* allocations have been de-allocated, the backing memory is made available again.
///
/// # Performance Note
/// The implementation internally uses a [`SpinLock`] to achieve interior thread-safe mutability which is needed
/// for some atomicity requirements involving [`allocate`](BumpAllocator::allocate) and [`deallocate`](BumpAllocator::deallocate).
///
/// This impacts performance when allocating and deallocating memory in parallel but  the lock is never returned to the
/// user so that a timely unlock is always ensured.
#[derive(Debug)]
pub struct BumpAllocator<'mem> {
    state: SpinLock<AllocatorState<'mem>>,
}

impl<'mem> BumpAllocator<'mem> {
    /// Create a new allocator that allocates from the memory region between `start` and `end`
    ///
    /// # Safety
    /// The entire memory area must be accessible and otherwise completely unused.
    pub unsafe fn new_raw(start: *mut u8, end: *mut u8) -> Self {
        assert!(start <= end);
        Self {
            state: SpinLock::new(AllocatorState {
                backing_mem: &mut *ptr::slice_from_raw_parts_mut(
                    start,
                    end as usize - start as usize,
                ),
                num_allocations: 0,
                bytes_allocated: 0,
            }),
        }
    }

    /// How much free space (in bytes) remains in the allocators backing memory
    pub fn capacity(&self) -> usize {
        let state = self.state.spin_lock();
        state.backing_mem.len() - state.bytes_allocated
    }

    /// Steal the remaining free memory from the allocator.
    ///
    /// This makes the stolen memory unavailable to the allocator so that no further regions are allocated from it.
    pub fn steal_remaining_mem(&self) -> &'mem mut [u8] {
        let mut state = self.state.spin_lock();

        let mut dummy = [0u8; 0].as_mut_slice();
        mem::swap(&mut dummy, &mut state.backing_mem);

        let mut split = dummy.split_at_mut(state.bytes_allocated);
        mem::swap(&mut split.0, &mut state.backing_mem);
        split.1
    }

    /// Allocate a slice of the given size aligned to `alignment` bytes.
    pub fn allocate<'alloc>(
        &'alloc self,
        size: usize,
        alignment: usize,
        init: AllocInit,
    ) -> Result<&'mem mut [u8], AllocError> {
        assert!(
            alignment.is_power_of_two(),
            "alignment must be a power of two"
        );
        assert!(size > 0, "must allocate at least 1 byte");

        let result = {
            let mut state = self.state.spin_lock();

            unsafe {
                let unaligned_ptr =
                    state.backing_mem.as_mut_ptr().add(state.bytes_allocated) as usize;
                let aligned_ptr = (unaligned_ptr + alignment - 1) & !(alignment - 1);
                let bytes_to_allocate = aligned_ptr - unaligned_ptr + size;

                // check that there even is enough space to allocate the requested amount
                if state
                    .backing_mem
                    .len()
                    .saturating_sub(state.bytes_allocated)
                    < bytes_to_allocate
                {
                    return Err(AllocError::InsufficientMemory);
                }

                // update state to include the now allocated bytes
                state.num_allocations += 1;
                state.bytes_allocated += bytes_to_allocate;

                // carve out a subslice from the backing memory.
                // this is unsafe because rust aliasing rules only allow one mutable reference to exist for the backing memory
                // slice but we give out multiple.
                // however the allocator ensures that slices don't overlap via the `bytes_allocated` counter which makes this
                // safe to do
                &mut *ptr::slice_from_raw_parts_mut(aligned_ptr as *mut u8, size)
            }
        };

        // initialize memory if required
        match init {
            AllocInit::Zeroed => result.fill(0),
            AllocInit::Data(data) => result.fill(data),
            AllocInit::Uninitialized => {}
        }

        Ok(result)
    }

    /// Deallocate the given data.
    ///
    /// # Panics
    /// This function panics if the given `data_ptr` does not lie within the bounds of the allocators backing memory.
    ///
    /// # Safety
    /// The given data must be *currently allocated* from this allocator.
    ///
    /// This means that:
    /// - it was previously returned by [`allocate`](BumpAllocator::allocate)
    /// - it has not yet been deallocated
    pub unsafe fn deallocate(&self, data_ptr: *mut u8) {
        let mut state = self.state.spin_lock();
        assert!(data_ptr as usize >= state.backing_mem.as_ptr() as usize, "deallocate was called with a data_ptr that does not point inside the allocators backing memory");
        assert!((data_ptr as usize) < state.backing_mem.as_ptr() as usize + state.backing_mem.len(), "deallocate was called with a data_ptr that does not point inside the allocators backing memory");

        // update state to reflect the de-allocation
        state.num_allocations -= 1;

        // reset allocation marker if there is now nothing allocated
        // this effectively makes all memory available again for allocation
        if state.num_allocations == 0 {
            state.bytes_allocated = 0;
        }
    }
}
