use crate::cap_counted::CapCounted;
use crate::{Correspondence, TreeNodeData, TreeNodeOps};
use allocators::{AllocError, AllocInit, Allocator, Box};
use core::alloc::Layout;
use core::mem::ManuallyDrop;

/// A capability for managing memory
pub struct Memory<
    'mem,
    'allocator,
    SourceAllocator: Allocator<'mem>,
    ContentAllocator: Allocator<'mem>,
    T: TreeNodeOps,
> {
    pub tree_data: TreeNodeData<T>,
    allocator: CapCounted<'allocator, 'mem, SourceAllocator, ContentAllocator>,
    backing_mem: CapCounted<'allocator, 'mem, SourceAllocator, [u8]>,
}

impl<'mem, SourceAllocator: Allocator<'mem>, ContentAllocator: Allocator<'mem>, T: TreeNodeOps>
    Memory<'mem, '_, SourceAllocator, ContentAllocator, T>
{
    /// Create a new Memory capability by allocating space from an existing source allocator.
    ///
    /// `size` is the number of bytes that should be made available via the newly created instance.
    ///
    /// # Safety
    /// The returned capability object is not yet part of a derivation tree and must be added to one before usage.
    pub unsafe fn alloc_new(
        source_allocator: &SourceAllocator,
        size: usize,
        alloc_init: impl FnOnce(&'mem mut [u8]) -> ContentAllocator,
    ) -> Result<Self, AllocError> {
        let mut backing_mem = Box::new_uninit_slice(size, source_allocator)?.assume_init();
        let allocator = Box::new(
            alloc_init(unsafe { &mut *(&mut backing_mem as *mut [u8]) }),
            source_allocator,
        )?;

        Ok(Self {
            tree_data: TreeNodeData::new(),
            allocator: allocator.into(),
            backing_mem: backing_mem.into(),
        })
    }

    pub unsafe fn destroy(self_node: &T) {
        todo!()
    }
}

impl<'mem, SourceAllocator: Allocator<'mem>, ContentAllocator: Allocator<'mem>, T: TreeNodeOps>
    Correspondence for Memory<'mem, '_, SourceAllocator, ContentAllocator, T>
{
    fn corresponds_to(&self, other: &Self) -> bool {
        self.allocator.is_same_pointer_as(other)
    }
}
