use crate::mem::page_table_entry::PageTableEntry;
use crate::mem::{MemoryPage, PAGESIZE};
use allocators::bump_allocator::{BumpAllocator, BumpBox};
use allocators::AllocFailed;
use core::mem::MaybeUninit;
use static_assertions::{assert_eq_align, assert_eq_size};

/// A PageTable for configuring virtual memory mapping.
///
/// It exactly fills 4096 bytes which is also the size of mapped pages.
#[derive(Debug)]
#[repr(C, align(4096))]
pub struct PageTable {
    pub entries: [PageTableEntry; 512],
}

assert_eq_size!(PageTable, MemoryPage);
assert_eq_align!(PageTable, MemoryPage);

impl PageTable {
    /// Create a new empty PageTable
    pub fn new<'mem, 'alloc, A: BumpAllocator<'mem>>(
        allocator: &'alloc A,
    ) -> Result<BumpBox<'alloc, 'mem, A, Self>, AllocFailed> {
        // this is safe because zero-initialized memory is a valid PageTable
        Ok(unsafe { BumpBox::new_zeroed(allocator)?.assume_init() })
    }

    // TODO Maybe rework this api to be safer
    // This doesn't do a deep copy, so it should only be used for global mappings
    pub fn init_copy(page: *mut MaybeUninit<MemoryPage>, orig: &PageTable) -> *mut PageTable {
        log::debug!("unit page: {page:p}, orig: {orig:p}");
        let root = PageTable::init(page);
        let root_ref = unsafe { root.as_mut().unwrap() };
        for (i, &entry) in orig.entries.iter().enumerate() {
            if entry.is_valid() {
                root_ref.entries[i] = entry;
            }
        }
        return root;
    }

    // TODO Does this need to be public?
    pub fn init(page: *mut MaybeUninit<MemoryPage>) -> *mut PageTable {
        unsafe {
            for i in 0..PAGESIZE {
                *page.cast::<u8>().add(i) = 0;
            }
        }
        page.cast::<PageTable>()
    }
}
