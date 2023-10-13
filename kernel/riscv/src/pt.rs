//! a module for RISCV Page Tables

use bitflags::bitflags;
use core::fmt::Debug;
use core::fmt::Write;
use core::mem::MaybeUninit;
use static_assertions::{assert_eq_align, assert_eq_size};

/// How large each memory page is
///
/// This effects the alignment and sizes of some data structures that directly interface with the CPU e.g. PageTables
pub const PAGESIZE: usize = 4096;

// TODO Refactor these variable to be more descriptive
const PBITS: usize = 12; // the page offset is 12 bits long
const PBIT_MASK: usize = (1 << PBITS) - 1;
// const PPN_BITS: usize = 56;
// const PADDR_MASK: usize = (1 << PPN_BITS) - 1;

// For Sv39 and Sv48, each VPN section has 9 bits in length;
// const VPN_BITS: usize = 9;
// const VPN_MASK: usize = (1 << VPN_BITS) - 1;

/// Type definition for a slice of bytes that is exactly one page large
#[repr(C, align(4096))]
pub struct MemoryPage([u8; PAGESIZE]);

/// An entry of a [`PageTable`](PageTable) responsible for mapping virtual to phyiscal adresses.
#[derive(Copy, Clone)]
pub struct PageTableEntry {
    entry: u64,
}

/// A PageTable for configuring virtual memory mapping.
///
/// It exactly fills 4096 bytes which is also the size of mapped pages.
#[repr(C, align(4096))]
pub struct PageTable {
    pub entries: [PageTableEntry; 512],
}

assert_eq_size!(PageTable, MemoryPage);
assert_eq_align!(PageTable, MemoryPage);

impl PageTable {
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

#[derive(Debug)]
pub struct EntryInvalidErr();

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

    /// Set the content of this entry.
    ///
    /// Since function also automatically enables the entry by setting the [`Valid`](EntryFlags::Valid) flag.
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
            (self as *mut Self as usize) & !(PAGESIZE - 1),
            ((self as *mut Self as usize) & (PAGESIZE - 1)) / 8,
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

    /// Whether this entry is currently valid (in other words whether it is considered active)
    pub fn is_valid(&self) -> bool {
        self.get_flags().contains(EntryFlags::Valid)
    }

    /// Whether this is a leaf entry not pointing to further [`PageTable`]s.
    pub fn is_leaf(&self) -> bool {
        self.get_flags().intersects(EntryFlags::RWX)
    }
}

const PPN_BITS: usize = 56;
const PADDR_MASK: usize = (1 << PPN_BITS) - 1;

// For Sv39 and Sv48, each VPN section has 9 bits in length;
const VPN_BITS: usize = 9;
const VPN_MASK: usize = (1 << VPN_BITS) - 1;

fn vpn_segments(vaddr: usize) -> [usize; 3] {
    let vpn = [
        (vaddr >> (PBITS + 0 * VPN_BITS)) & VPN_MASK,
        (vaddr >> (PBITS + 1 * VPN_BITS)) & VPN_MASK,
        (vaddr >> (PBITS + 2 * VPN_BITS)) & VPN_MASK,
        // if Sv48, there is a level of page tables more
        // (vaddr >> (12 + 3 * VPN_BITS)) & VPN_BIT_MASK,
    ];
    vpn
}

#[derive(Debug)]
pub struct MissingPageErr {
    pub level: usize,
    pub target_vaddr: usize,
}

pub fn map_pt(
    trans: impl crate::PhysMapper,
    root: &mut PageTable,
    target_level: usize,
    vaddr: usize,
    pt: *mut PageTable,
) -> Result<(), MissingPageErr> {
    log::debug!("[map_pt] root: {root:p} vaddr: {vaddr:#x} level: {target_level}");
    let vpn = vpn_segments(vaddr);

    let mut v = &mut root.entries[vpn[2]];
    for level in (0..2).rev() {
        if level == target_level {
            unsafe { v.set(trans.mapped_to_phys_mut(pt) as u64, EntryFlags::Valid) };
            return Ok(());
        }
        if !v.is_valid() {
            return Err(MissingPageErr {
                level,
                target_vaddr: vaddr,
            });
        }
        let next_pt_addr = v.get_addr().unwrap();
        let next_pt = unsafe { trans.phys_to_mapped_mut(next_pt_addr as *mut PageTable) };
        v = unsafe { &mut (*next_pt).entries[vpn[level]] };
    }

    panic!("can't set leaf entry as pt");
}

pub fn unmap(trans: impl crate::PhysMapper, root: &mut PageTable, vaddr: usize, paddr: usize) {
    // physical address should be at least page aligned and in PPN range
    assert!(paddr & PBIT_MASK == 0);
    assert!(paddr & !PADDR_MASK == 0);
    // virtual addresses must also be page aligned
    assert!(vaddr & PBIT_MASK == 0, "vaddr is not page-aligned");

    let vpn = vpn_segments(vaddr);

    let mut v = &mut root.entries[vpn[2]];
    for level in (0..2).rev() {
        if !v.is_valid() {
            log::warn!("missing pt when unmapping page");
            return;
        }
        let next_pt_addr = v.get_addr().unwrap();
        let next_pt = unsafe { trans.phys_to_mapped_mut(next_pt_addr as *mut PageTable) };
        v = unsafe { &mut (*next_pt).entries[vpn[level]] };
    }

    if !v.is_valid() {
        log::warn!("tried to unmap invalid page entry");
        return;
    }
    if !v.is_leaf() {
        log::error!("tried to unmap page entry that is not a leaf");
        panic!();
    }
    if v.get_addr().unwrap() != paddr {
        log::warn!("tried to unmap page at vaddr that currently holds another page: entry {:x} paddr: {:x}", v.get_addr().unwrap(), paddr);
        return;
    }
    unsafe {
        v.set(0 as u64, EntryFlags::empty());
    }
}

pub fn map(
    trans: impl crate::PhysMapper,
    root: &mut PageTable,
    vaddr: usize,
    paddr: usize,
    flags: EntryFlags,
) -> Result<(), MissingPageErr> {
    log::debug!("[map] root: {root:p} vaddr: {vaddr:#x} paddr: {paddr:#x} flags: {flags:?}");
    // Make sure that one of Read, Write, or Execute Bits is set.
    // Otherwise, entry is regarded as pointer to next page table level
    assert_eq!(flags.bits() & EntryFlags::all().bits(), flags.bits());
    assert_ne!((flags & EntryFlags::RWX), EntryFlags::empty());

    // physical address should be at least page aligned and in PPN range
    assert!(paddr & PBIT_MASK == 0);
    assert!(paddr & !PADDR_MASK == 0);
    // virtual addresses must also be page aligned
    assert!(vaddr & PBIT_MASK == 0, "vaddr is not page-aligned");

    let vpn = vpn_segments(vaddr);

    let mut v = &mut root.entries[vpn[2]];
    for level in (0..2).rev() {
        if !v.is_valid() {
            return Err(MissingPageErr {
                level,
                target_vaddr: vaddr,
            });
        }
        let next_pt_addr = v.get_addr().unwrap();
        let next_pt = unsafe { trans.phys_to_mapped_mut(next_pt_addr as *mut PageTable) };
        v = unsafe { &mut (*next_pt).entries[vpn[level]] };
    }

    // Now we are ready to point v to our physical address
    assert!(
        !v.is_valid(),
        "the pagetable entry is already mapped. maybe you're trying to map overlapping pages"
    );
    unsafe {
        v.set(paddr as u64, flags | EntryFlags::Valid);
    }
    Ok(())
}

pub fn virt_to_phys(
    trans: impl crate::PhysMapper,
    root: &PageTable,
    vaddr: usize,
) -> Option<usize> {
    let vpn = vpn_segments(vaddr);
    let v = &root.entries[vpn[2]];
    if !v.is_valid() {
        return None;
    }
    if v.is_leaf() {
        panic!("hugepage encountered");
    }
    let pt = unsafe {
        let next_pt_addr = v.get_addr().unwrap();
        let next_pt = trans.phys_to_mapped(next_pt_addr as *const PageTable);
        &*next_pt
    };
    let v = &pt.entries[vpn[1]];
    if !v.is_valid() {
        return None;
    }
    if v.is_leaf() {
        panic!("hugepage encountered");
    }
    let pt = unsafe {
        let next_pt_addr = v.get_addr().unwrap();
        let next_pt = trans.phys_to_mapped(next_pt_addr as *const PageTable);
        &*next_pt
    };
    let v = &pt.entries[vpn[0]];
    if !v.is_valid() {
        return None;
    }
    if !v.is_leaf() {
        panic!("non leaf page where leaf was expected");
    }

    let address = v.get_addr().unwrap();
    Some(address | (vaddr & PBIT_MASK))
}
