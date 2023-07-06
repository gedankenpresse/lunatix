use crate::mem::page_table_entry::PageTableEntry;
use crate::mem::MemoryPage;
use allocators::bump_allocator::{BumpAllocator, BumpBox};
use allocators::AllocFailed;
use static_assertions::assert_eq_size;

/// A PageTable for configuring virtual memory mapping.
///
/// It exactly fills 4096 bytes which is also the size of mapped pages.
#[derive(Debug)]
pub struct PageTable {
    pub entries: [PageTableEntry; 512],
}

assert_eq_size!(PageTable, MemoryPage);

impl PageTable {
    /// Create a new empty PageTable
    pub fn new<'mem, 'alloc, A: BumpAllocator<'mem>>(
        allocator: &'alloc A,
    ) -> Result<BumpBox<'alloc, 'mem, A, Self>, AllocFailed> {
        // this is safe because zero-initialized memory is a valid PageTable
        Ok(unsafe { BumpBox::new_zeroed(allocator)?.assume_init() })
    }
}
