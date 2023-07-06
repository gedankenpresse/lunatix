use bitflags::bitflags;
use core::fmt::{Debug, Write};
use thiserror_no_std::Error;

// TODO Refactor these variable to be more descriptive
const PBITS: usize = 12; // the page offset is 12 bits long
const PBIT_MASK: usize = (1 << PBITS) - 1;
// const PPN_BITS: usize = 56;
// const PADDR_MASK: usize = (1 << PPN_BITS) - 1;

// For Sv39 and Sv48, each VPN section has 9 bits in length;
// const VPN_BITS: usize = 9;
// const VPN_MASK: usize = (1 << VPN_BITS) - 1;

#[derive(Debug, Error)]
#[error("The PageTableEntry is not set as valid")]
pub struct EntryInvalidErr();

/// An entry of a [`PageTable`](PageTable) responsible for mapping virtual to phyiscal adresses.
#[derive(Copy, Clone)]
pub struct PageTableEntry {
    entry: u64,
}

impl PageTableEntry {
    /// Return the flags which are encoded in this entry
    pub fn get_flags(&self) -> EntryFlags {
        EntryFlags::from_bits_truncate(self.entry)
    }

    /// Return the address which this entry points to
    pub fn get_addr(&self) -> Result<usize, EntryInvalidErr> {
        if self.is_valid() {
            // TODO: Is this correct?
            Ok(((self.entry << 2) & !PBIT_MASK as u64) as usize)
        } else {
            Err(EntryInvalidErr())
        }
    }

    /// Set the content of this entry
    ///
    /// # Safety
    /// Changing the entry of a PageTable inherently changes virtual address mappings.
    /// This can make other, completely unrelated, references and pointers invalid and must always be done with
    /// care.
    pub unsafe fn set(&mut self, paddr: u64, flags: EntryFlags) {
        // TODO: Fix that an unaligned paddr leaks into flags
        self.entry = (paddr >> 2) | (flags | EntryFlags::Valid).bits();
    }

    /// Whether this entry is currently valid (in other words whether it is considered active)
    pub fn is_valid(&self) -> bool {
        self.get_flags().contains(EntryFlags::Valid)
    }

    /// Whether this is a leaf entry not pointing to further [`PageTable`]s.
    pub fn is_leaf(&self) -> bool {
        self.get_flags().intersects(EntryFlags::RWX)
    }
}

impl Debug for PageTableEntry {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut debug = f.debug_struct("Entry");

        match self.get_addr() {
            Err(_) => debug.field("ppn", &"invalid"),
            Ok(addr) => debug.field("ppn", &format_args!("{:#x}", addr)),
        };

        debug.field("flags", &self.get_flags()).finish()
    }
}

bitflags! {
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
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        fn write_bit(
            flags: EntryFlags,
            bit: EntryFlags,
            c: char,
            f: &mut core::fmt::Formatter<'_>,
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
