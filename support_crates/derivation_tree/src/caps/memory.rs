use crate::cap_counted::CapCounted;
use crate::Correspondence;
use allocators::{AllocError, Allocator, Box};
use core::ops::DerefMut;

/// A capability for managing memory
pub struct Memory<
    'mem,
    'allocator,
    SourceAllocator: Allocator<'mem>,
    ContentAllocator: Allocator<'mem>,
> {
    pub(crate) allocator: CapCounted<'allocator, 'mem, SourceAllocator, ContentAllocator>,
    backing_mem: CapCounted<'allocator, 'mem, SourceAllocator, [u8]>,
}

impl<'mem, 'allocator, SourceAllocator: Allocator<'mem>, ContentAllocator: Allocator<'mem>>
    Memory<'mem, 'allocator, SourceAllocator, ContentAllocator>
{
    /// Create a new Memory capability by allocating space from an existing source allocator.
    ///
    /// `size` is the number of bytes that should be made available via the newly created instance.
    ///
    /// # Safety
    /// The returned capability object is not yet part of a derivation tree and must be added to one before usage.
    pub unsafe fn alloc_new(
        source_allocator: &'allocator SourceAllocator,
        size: usize,
        alloc_init: impl FnOnce(&'mem mut [u8]) -> ContentAllocator,
    ) -> Result<Self, AllocError> {
        let mut backing_mem = Box::new_uninit_slice(size, source_allocator)?.assume_init();
        let allocator = Box::new(
            alloc_init(unsafe { &mut *(backing_mem.deref_mut() as *mut [u8]) }),
            source_allocator,
        )?;

        Ok(Self {
            allocator: allocator.into(),
            backing_mem: backing_mem.into(),
        })
    }

    /// Deallocate the backing memory of this memory capability.
    ///
    /// # Safety
    /// This method must only be called once and only on the last existing capability copy.
    pub unsafe fn deallocate(&mut self) {
        self.backing_mem.destroy();
        self.allocator.destroy();
    }
}

impl<'mem, SourceAllocator: Allocator<'mem>, ContentAllocator: Allocator<'mem>> Correspondence
    for Memory<'mem, '_, SourceAllocator, ContentAllocator>
{
    fn corresponds_to(&self, other: &Self) -> bool {
        self.allocator.is_same_pointer_as(&other.allocator)
    }
}
