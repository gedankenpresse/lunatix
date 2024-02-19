use super::PAGESIZE;
use crate::mem::paddr;
use crate::mem::paddr::PAddr;
use bitflags::{bitflags, Flags};
use core::fmt::{Binary, Debug, Formatter, LowerHex, UpperHex, Write};
use core::mem::MaybeUninit;

/// An entry of a [`PageTable`](PageTable) responsible for mapping virtual to phyiscal adresses.
///
/// # Format
/// The PTE format for Sv39 is shown in the below figure.
/// - Bits 7–0 have the meaning as described by [`EntryFlags`].
/// - Bits 7-9 are ignored by the hardware implementation and can be freely used by sofware.
/// - Bit 63 is reserved for use by the Svnapot extension. If Svnapot is not implemented, bit 63 remains reserved and must be zeroed by software for forward compatibility, or else a page-fault exception is raised.
/// - Bits 62–61 are reserved for use by the Svpbmt extension. If Svpbmt is not implemented, bits 62–61 remain reserved and must be zeroed by software for forward compatibility, or else a page-fault exception is raised.
/// - Bits 60–54 are reserved for future standard use and, until their use is defined by some standard extension, must be zeroed by software for forward compatibility. If any of these bits are set, a page-fault exception is raised.
///
/// ```text
///   63 62  61 60      54 53    28 27    19 18    10 9   8  7   6   5   4   3   2   1   0
/// ┌───┬──────┬──────────┬────────┬────────┬────────┬─────┬───┬───┬───┬───┬───┬───┬───┬───┐
/// │ N │ PBMT │ reserved │ PPN[2] │ PPN[1] │ PPN[0] │ RSW │ D │ A │ G │ U │ X │ W │ R │ V │
/// └───┴──────┴──────────┴────────┴────────┴────────┴─────┴───┴───┴───┴───┴───┴───┴───┴───┘
///       2bit     7bit     26bit     9bit     9bit   2bit
///                      Sv39 Page Table Entry
/// ```
///
#[derive(Eq, PartialEq)]
#[repr(C, align(8))]
pub struct PageTableEntry {
    pub(crate) entry: u64,
}

const FLAG_BITS: u64 = 8;
const FLAG_MASK: u64 = (1 << FLAG_BITS) - 1;
const PPN_OFFSET: u64 = 10;
const PPN_BITS: u64 = 44;
const PPN_MASK: u64 = ((1 << PPN_BITS) - 1) << PPN_OFFSET;

impl PageTableEntry {
    /// Initialize the page table entry located at `ptr` to be empty and not point to anything
    pub(crate) fn init_empty(ptr: *mut MaybeUninit<PageTableEntry>) -> *mut PageTableEntry {
        let ptr = ptr.cast::<PageTableEntry>();
        unsafe { ptr.write(Self { entry: 0 }) };
        ptr
    }

    /// Initialize the page table entry located at `ptr` with the given data
    pub(crate) fn init(
        ptr: *mut MaybeUninit<PageTableEntry>,
        addr: PAddr,
        flags: EntryFlags,
    ) -> *mut PageTableEntry {
        let ptr = Self::init_empty(ptr);
        let pte = unsafe { ptr.as_mut().unwrap() };
        unsafe { pte.set(addr, flags) };
        ptr
    }

    /// Create a new empty entry.
    ///
    /// This entry does not point to anything and is considered disabled by the hardware.
    #[deprecated(note = "use init_empty() instead")]
    pub(crate) fn empty() -> Self {
        Self { entry: 0 }
    }

    /// Create a new entry that contains the given data
    #[deprecated(note = "use init() instead")]
    pub(crate) fn new(addr: PAddr, flags: EntryFlags) -> Self {
        let mut entry = Self::empty();
        unsafe { entry.set(addr, flags) };
        entry
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
        EntryFlags::from_bits_truncate(self.entry & FLAG_MASK)
    }

    /// Return the address which this entry points to
    pub fn get_addr(&self) -> Result<PAddr, EntryInvalidErr> {
        match self.is_valid() {
            false => Err(EntryInvalidErr),
            true => Ok((self.entry & PPN_MASK) >> PPN_OFFSET << paddr::PPN_OFFSET),
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
    pub unsafe fn set(&mut self, addr: PAddr, flags: EntryFlags) {
        assert_eq!(
            addr & paddr::PAGE_OFFSET_MASK,
            0,
            "cannot set page table entry to unaligned PAddr {:#x}",
            addr
        );
        log::trace!(
            "setting page table entry {:#x}:{} to {:#x} with flags {flags:?}",
            (self as *mut _ as u64) & paddr::PPN_MASK,
            ((self as *mut _ as u64) & paddr::PAGE_OFFSET_MASK)
                / core::mem::size_of::<PageTableEntry>() as u64,
            addr
        );

        self.entry = (((addr & paddr::PPN_MASK) >> paddr::PPN_OFFSET) << PPN_OFFSET)
            | (flags | EntryFlags::Valid | EntryFlags::Dirty | EntryFlags::Accessed).bits();
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

impl Debug for PageTableEntry {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let table_addr = (self as *const Self as usize) & !(PAGESIZE - 1);
        let entry_index = ((self as *const Self as usize) & (PAGESIZE - 1)) / 8;
        match self.get_addr() {
            Err(_) => f.write_fmt(format_args!(
                "PageTableEntry {:#x}:{:03} (invalid) {{ .. }}",
                table_addr, entry_index
            )),
            Ok(addr) => f.write_fmt(format_args!(
                "PageTableEntry {:#x}:{:03} {{ addr: {:12x}, flags: {:?} }}",
                table_addr,
                entry_index,
                addr,
                self.get_flags()
            )),
        }
    }
}

impl Binary for PageTableEntry {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        Binary::fmt(&self.entry, f)
    }
}

impl LowerHex for PageTableEntry {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        LowerHex::fmt(&self.entry, f)
    }
}

impl UpperHex for PageTableEntry {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        UpperHex::fmt(&self.entry, f)
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_address_is_correctly_loaded() {
        let entry = PageTableEntry {
            entry: 0x3FFFFFFFFFFC01u64,
        };
        assert_eq!(entry.get_addr().unwrap(), 0x3FFFFFFFFFFC00 << 2);
    }

    #[test]
    fn test_address_is_correctly_set() {
        let mut entry = PageTableEntry { entry: 0 };
        unsafe { entry.set(0x80042000, EntryFlags::Valid) };
        assert_eq!(
            entry.entry,
            (0x80042000 >> 2) | 0x1,
            "{:#x} != {:#x}",
            entry.entry,
            (0x80042000u64 >> 2) | 0x1
        )
    }
}
