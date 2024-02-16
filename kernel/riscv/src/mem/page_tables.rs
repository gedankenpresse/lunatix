use crate::mem::{MemoryPage, PageTableEntry, PAGESIZE};
use core::fmt::{Binary, Debug, Display, Formatter};
use core::mem;
use core::mem::MaybeUninit;
use static_assertions::{assert_eq_align, assert_eq_size};

// TODO Refactor these variable to be more descriptive
const PBITS: usize = 12; // the page offset is 12 bits long
const PBIT_MASK: usize = (1 << PBITS) - 1;

/// A PageTable for configuring virtual memory mapping.
///
/// It exactly fills 4096 bytes which is also the size of mapped pages.
#[repr(C, align(4096))]
#[derive(Eq, PartialEq)]
pub struct PageTable {
    pub entries: [PageTableEntry; PAGESIZE / mem::size_of::<PageTableEntry>()],
}

assert_eq_size!(PageTable, MemoryPage);
assert_eq_align!(PageTable, MemoryPage);

impl PageTable {
    // TODO Maybe this should use pins instead of pointers
    /// Initialize the given page with an empty `PageTable`
    pub fn init(page: *mut MaybeUninit<MemoryPage>) -> *mut PageTable {
        log::trace!("initializing empty pagetable at {page:p}");
        for i in 0..PAGESIZE / mem::size_of::<PageTableEntry>() {
            unsafe {
                page.cast::<PageTableEntry>()
                    .add(i)
                    .write(PageTableEntry::empty());
            }
        }

        page.cast::<PageTable>()
    }

    /// Initialize the given page to contain a copy of another `PageTable`
    ///
    /// Note that this doesn't do a deep copy.
    /// All page tables further down the reference tree are not copied but instead reused.
    /// This means that this function should only be used for global mappings.
    #[deprecated(note = "use init_copy_high() instead")]
    pub fn init_copy(page: *mut MaybeUninit<MemoryPage>, orig: &PageTable) -> *mut PageTable {
        log::trace!("initializing pagetable at {page:p} to be a copy of {orig:p}");

        let table = PageTable::init(page);
        let table_ref = unsafe { table.as_mut().unwrap() };
        for (i, entry) in orig.entries.iter().enumerate() {
            if entry.is_valid() {
                table_ref.entries[i] = PageTableEntry::new(entry.entry);
            }
        }

        return table;
    }

    /// Initialize the given page as a `PageTable` that is empty in its lower address space and contains the same data as `orig` in its high address space.
    ///
    /// For the definition of low and high address space see the module level documentation.
    pub fn init_copy_high(page: *mut MaybeUninit<MemoryPage>, orig: &PageTable) -> *mut PageTable {
        log::trace!("initializing pagetable at {page:p} with same high-address space as {orig:p}");

        for i in 0..PAGESIZE / mem::size_of::<PageTableEntry>() {
            todo!("initialize lower half with empty and higher half with copies")
        }

        page.cast::<PageTable>()
    }
}

impl Debug for PageTable {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!(
            "PageTable 0x{:X} {{\n",
            self as *const _ as usize
        ))?;

        for (i, entry) in self.entries.iter().enumerate() {
            if entry.is_valid() || f.alternate() {
                f.write_fmt(format_args!("  {i:3}: {:?}\n", entry))?;
            }
        }

        f.write_str("}")?;
        Ok(())
    }
}
