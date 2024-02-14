use bitflags::bitflags;
use core::fmt::{Debug, Display, Formatter, Write};
use core::mem;
use core::mem::MaybeUninit;
use core::ops::{Deref, DerefMut};
use static_assertions::{assert_eq_align, assert_eq_size};

// TODO Refactor these variable to be more descriptive
const PBITS: usize = 12; // the page offset is 12 bits long
const PBIT_MASK: usize = (1 << PBITS) - 1;

/// How large each page in the memory of a riscv board is.
///
/// This effects the alignment and sizes of some data structures that directly interface with the CPU e.g. PageTables
pub const PAGESIZE: usize = 4096;

/// Type definition for a slice of bytes that is exactly one page large and aligned to it as well
#[derive(Copy, Clone, Eq, PartialEq)]
#[repr(C, align(4096))]
pub struct MemoryPage([u8; PAGESIZE]);

impl Deref for MemoryPage {
    type Target = [u8; PAGESIZE];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for MemoryPage {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Default for MemoryPage {
    fn default() -> Self {
        Self([0u8; PAGESIZE])
    }
}

/// A PageTable for configuring virtual memory mapping.
///
/// It exactly fills 4096 bytes which is also the size of mapped pages.
#[repr(C, align(4096))]
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
        for (i, &entry) in orig.entries.iter().enumerate() {
            if entry.is_valid() {
                table_ref.entries[i] = entry;
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
            // TODO
            unsafe {
                page.cast::<PageTableEntry>()
                    .add(i)
                    .write(PageTableEntry::empty());
            }
        }

        page.cast::<PageTable>()
    }
}

/// An entry of a [`PageTable`](PageTable) responsible for mapping virtual to phyiscal adresses.
#[derive(Copy, Clone, Eq, PartialEq)]
#[repr(C, align(8))]
pub struct PageTableEntry {
    entry: u64,
}

impl PageTableEntry {
    /// Create a new empty entry.
    ///
    /// This entry does not point to anything and is considered disabled by the hardware.
    pub fn empty() -> Self {
        Self { entry: 0 }
    }

    /// Whether this entry is currently valid (in other words whether it is considered active)
    pub fn is_valid(&self) -> bool {
        self.get_flags().contains(EntryFlags::Valid)
    }

    /// Whether this is a leaf entry not pointing to further [`PageTable`]s.
    pub fn is_leaf(&self) -> bool {
        self.get_flags().intersects(EntryFlags::RWX)
    }

    /// Return the flags which are encoded in this entry
    pub fn get_flags(&self) -> EntryFlags {
        EntryFlags::from_bits_truncate(self.entry)
    }

    /// Return the address which this entry points to
    pub fn get_addr(&self) -> Result<usize, EntryInvalidErr> {
        match self.is_valid() {
            false => Err(EntryInvalidErr),
            true =>
            // TODO: Is this correct?
            {
                Ok(((self.entry << 2) & !PBIT_MASK as u64) as usize)
            }
        }
    }

    /// Set the content of this entry.
    ///
    /// This function also automatically enables the entry by setting the [`Valid`](EntryFlags::Valid) flag.
    ///
    /// If you want to disable the entry use [`clear()`](PageTableEntry::clear) instead.
    ///
    /// # Safety
    /// Changing the entry of a PageTable inherently changes virtual address mappings.
    /// This can make other, completely unrelated, references and pointers invalid and must always be done with
    /// care.
    pub unsafe fn set(&mut self, paddr: u64, flags: EntryFlags) {
        log::trace!(
            "setting page table entry {:#x}:{} to {:#x}",
            (self as *mut _ as usize) & !(PAGESIZE - 1),
            ((self as *mut _ as usize) & (PAGESIZE - 1)) / mem::size_of::<PageTableEntry>(),
            paddr
        );

        // TODO: Fix that an unaligned paddr leaks into flags
        self.entry = (paddr >> 2) | (flags | EntryFlags::Valid).bits();
    }

    /// Clear the content of this entry, setting it to 0x0 and removing all flags.
    ///
    /// # Safety
    /// Changing the entry of a PageTable inherently changes virtual address mappings.
    /// This can make other, completely unrelated, references and pointers invalid and must always be done with
    /// care.
    pub unsafe fn clear(&mut self) {
        log::trace!(
            "clearing page table entry {:#x}:{}",
            (self as *mut Self as usize) & !(PAGESIZE - 1),
            ((self as *mut Self as usize) & (PAGESIZE - 1)) / 8,
        );

        self.entry = 0;
    }
}

bitflags! {
    /// The flags that can be set on a [`PageTableEntry`]
    #[derive(Copy, Clone, Eq, PartialEq)]
    pub struct EntryFlags: u64 {
        /// If set, the MMU considers this a valid entry in the page table and uses it for address mapping
        const Valid = 1 << 0;
        /// Allows reading from the mapped page
        const Read = 1 << 1;
        /// Allows writing from the mapped page
        const Write = 1 << 2;
        /// Allows executing code from the mapped page
        const Execute = 1 << 3;
        /// Allows reading from the mapped page **from user mode**
        const UserReadable = 1 << 4;
        /// If set, the MMU considers this entry to be present in **all** address space IDs and caches them accordingly.
        /// It is safe to never set this but when setting it, care should be taken to do it correctly.
        const Global = 1 << 5;
        /// Set by the MMU when something has read from the page since the mapping was set up
        const Accessed = 1 << 6;
        /// Set by the MMU when something has written to the page since the mapping was set up
        const Dirty = 1 << 7;

        /// Custom bit available for use by us
        const CUSTOM1 = 1 << 8;
        /// Custom bit available for use by us
        const CUSTOM2 = 1 << 9;

        const RWX = Self::Read.bits() | Self::Write.bits() | Self::Execute.bits();
    }
}

impl Debug for EntryFlags {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        fn write_bit(
            flags: EntryFlags,
            bit: EntryFlags,
            c: char,
            f: &mut Formatter<'_>,
        ) -> core::fmt::Result {
            if flags.contains(bit) {
                f.write_char(c)
            } else {
                f.write_char(' ')
            }
        }
        write_bit(*self, EntryFlags::CUSTOM2, '2', f)?;
        write_bit(*self, EntryFlags::CUSTOM1, '1', f)?;
        write_bit(*self, EntryFlags::Dirty, 'D', f)?;
        write_bit(*self, EntryFlags::Accessed, 'A', f)?;
        write_bit(*self, EntryFlags::Global, 'G', f)?;
        write_bit(*self, EntryFlags::UserReadable, 'U', f)?;
        write_bit(*self, EntryFlags::Execute, 'X', f)?;
        write_bit(*self, EntryFlags::Write, 'W', f)?;
        write_bit(*self, EntryFlags::Read, 'R', f)?;
        write_bit(*self, EntryFlags::Valid, 'V', f)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct EntryInvalidErr;
