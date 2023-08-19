use crate::bump_allocator::bump_alloc_trait::BumpAllocator;
use crate::{AllocError, AllocInit, Allocator};
use core::alloc::Layout;
use core::{mem, ptr};
use ksync::SpinLock;

#[derive(Debug, Eq, PartialEq, Hash)]
struct AllocatorState<'mem> {
    backing_mem: &'mem mut [u8],
    bytes_allocated: usize,
    num_allocations: usize,
}

/// A [`BumpAllocator`] which starts allocations from the beginning of the backing memory and bumps an allocation
/// marker forwards to track the already allocated memory.
///
/// ```text
///   ┌────────────────── backing memory ────────────────────┐
///   │                                                      │
/// [0xA, 0xA, 0xA, 0xA, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0]
///                  ^
///       marker ────┘
/// ```
///
/// # Performance Note
/// The implementation internally uses a [`SpinLock`] to achieve interior thread-safe mutability which is needed
/// for some atomicity requirements involving [`allocate`](ForwardBumpingAllocator::allocate) and [`deallocate`](ForwardBumpingAllocator::deallocate).
///
/// This impacts performance when allocating and deallocating memory in parallel but  the lock is never returned to the
/// user so that a timely unlock is always ensured.
#[derive(Debug)]
pub struct ForwardBumpingAllocator<'mem> {
    state: SpinLock<AllocatorState<'mem>>,
}

impl<'mem> Allocator<'mem> for ForwardBumpingAllocator<'mem> {
    fn allocate(&self, layout: Layout, init: AllocInit) -> Result<&'mem mut [u8], AllocError> {
        assert!(layout.size() > 0, "must allocate at least 1 byte");

        let result = {
            let mut state = self.state.spin_lock();

            unsafe {
                // TODO The Layout struct can calculate padding on its own. Maybe use that instead of doing it on our own
                let unaligned_ptr =
                    state.backing_mem.as_mut_ptr().add(state.bytes_allocated) as usize;
                let aligned_ptr = (unaligned_ptr + layout.align() - 1) & !(layout.align() - 1);
                let bytes_to_allocate = aligned_ptr - unaligned_ptr + layout.size();

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
                &mut *ptr::slice_from_raw_parts_mut(aligned_ptr as *mut u8, layout.size())
            }
        };

        log::trace!(
            "allocated {} bytes: {:p} -- {:p}",
            layout.size(),
            result.as_ptr(),
            unsafe { result.as_ptr().add(layout.size()) }
        );

        // initialize memory if required
        match init {
            AllocInit::Zeroed => result.fill(0),
            AllocInit::Data(data) => result.fill(data),
            AllocInit::Uninitialized => {}
        }

        Ok(result)
    }

    unsafe fn deallocate(&self, data_ptr: *mut u8, _layout: Layout) {
        let mut state = self.state.spin_lock();
        assert!(data_ptr as usize >= state.backing_mem.as_ptr() as usize, "deallocate was called with a data_ptr that does not point inside the allocators backing memory");
        assert!((data_ptr as usize) < state.backing_mem.as_ptr() as usize + state.backing_mem.len(), "deallocate was called with a data_ptr that does not point inside the allocators backing memory");

        // update state to reflect the de-allocation
        state.num_allocations -= 1;

        // reset allocation marker if there is now nothing allocated
        // this effectively makes all memory available again for allocation
        if state.num_allocations == 0 {
            log::trace!("all allocations have been returned, resetting bump allocator");
            state.bytes_allocated = 0;
        }
    }
}

impl<'mem> BumpAllocator<'mem> for ForwardBumpingAllocator<'mem> {
    fn new(backing_mem: &'mem mut [u8]) -> Self {
        Self {
            state: SpinLock::new(AllocatorState {
                backing_mem,
                num_allocations: 0,
                bytes_allocated: 0,
            }),
        }
    }

    fn steal_remaining_mem(&self) -> &'mem mut [u8] {
        let mut state = self.state.spin_lock();

        let mut dummy = [0u8; 0].as_mut_slice();
        mem::swap(&mut dummy, &mut state.backing_mem);

        let mut split = dummy.split_at_mut(state.bytes_allocated);
        mem::swap(&mut split.0, &mut state.backing_mem);
        split.1
    }

    fn get_free_bytes(&self) -> usize {
        let state = self.state.spin_lock();
        state.backing_mem.len() - state.bytes_allocated
    }
}
