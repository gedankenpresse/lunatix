use crate::mem::{MemoryPage, PageTableEntry, PAGESIZE};
use core::fmt::{Debug, Formatter};
use core::mem;
use core::mem::MaybeUninit;
use static_assertions::{assert_eq_align, assert_eq_size};

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
    pub fn init(target: *mut MaybeUninit<PageTable>) -> *mut PageTable {
        assert_eq!(target as usize % mem::align_of::<PageTable>(), 0);

        log::trace!("initializing empty pagetable at {target:p}");
        for i in 0..PAGESIZE / mem::size_of::<PageTableEntry>() {
            unsafe {
                let entry = target.cast::<MaybeUninit<PageTableEntry>>().add(i);
                PageTableEntry::init_empty(entry);
            }
        }

        target.cast::<PageTable>()
    }

    /// Initialize the given page to contain a copy of another `PageTable`
    ///
    /// Note that this doesn't do a deep copy.
    /// All page tables further down the reference tree are not copied but instead reused.
    /// This means that this function should only be used for global mappings.
    #[deprecated(note = "use init_copy_high() instead")]
    pub fn init_copy(target: *mut MaybeUninit<PageTable>, orig: &PageTable) -> *mut PageTable {
        log::trace!("initializing pagetable at {target:p} to be a copy of {orig:p}");

        let table = PageTable::init(target);
        for (i, i_entry) in orig.entries.iter().enumerate() {
            if i_entry.is_valid() {
                let target_slot = unsafe { target.cast::<MaybeUninit<PageTableEntry>>().add(i) };
                PageTableEntry::init(
                    target_slot,
                    i_entry.get_addr().unwrap(),
                    i_entry.get_flags(),
                );
            }
        }

        return table;
    }

    /// Initialize the given page as a `PageTable` that is empty in its lower address space and contains the same data as `orig` in its high address space.
    ///
    /// For the definition of low and high address space see the module level documentation.
    pub fn init_copy_high(page: *mut MaybeUninit<MemoryPage>, orig: &PageTable) -> *mut PageTable {
        log::trace!("initializing pagetable at {page:p} with same high-address space as {orig:p}");

        for _i in 0..PAGESIZE / mem::size_of::<PageTableEntry>() {
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

#[cfg(test)]
mod test {
    use super::*;
    extern crate alloc;

    #[test]
    fn test_pagetable_init_zeroes_memory() {
        let mut buf = MemoryPage([u8::MAX; 4096]);
        PageTable::init(buf.as_mut_ptr().cast());
        assert_eq!(*buf, [0u8; 4096]);
    }
}
