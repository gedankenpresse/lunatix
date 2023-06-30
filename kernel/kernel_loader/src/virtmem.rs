use crate::allocator::BumpAllocator;
use crate::arch::cpu::*;
use bitflags::{bitflags, Flags};
use core::fmt::Write;
use core::mem;
use core::mem::MaybeUninit;

const PAGESIZE: usize = 4096;

/// An entry of a page table responsible for mapping virtual to phyiscal adresses.
#[derive(Copy, Clone)]
pub struct Entry {
    entry: u64,
}

impl core::fmt::Debug for Entry {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Entry")
            .field("ppn", unsafe { &self.get_ptr() })
            .field("flags", &self.flags())
            .finish()
    }
}

/// One virtual memory mapping page.
///
/// It exactly fills 4096 bytes which is also the size of mapped pages.
#[derive(Debug)]
pub struct PageTable {
    pub entries: [Entry; 512],
}

impl PageTable {
    pub fn empty(alloc: &mut BumpAllocator) -> Option<*mut PageTable> {
        let page = alloc.alloc(mem::size_of::<PageTable>(), PAGESIZE)?.cast();
        Some(Self::init(page))
    }

    pub fn init(page: *mut MaybeUninit<PageTable>) -> *mut PageTable {
        unsafe {
            for i in 0..PAGESIZE {
                *page.cast::<u8>().add(i) = 0;
            }
        }
        page.cast::<PageTable>()
    }
}

bitflags! {
    #[derive(Copy, Clone, Eq, PartialEq)]
    pub struct EntryBits: u64 {
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

impl core::fmt::Debug for EntryBits {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        fn write_bit(flags: EntryBits, bit: EntryBits, c: char, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            if flags.contains(bit) {
                f.write_char(c)
            } else {
                f.write_char(' ')
            }
        }
        write_bit(*self, EntryBits::CUSTOM2, '2', f)?;
        write_bit(*self, EntryBits::CUSTOM1, '1', f)?;
        write_bit(*self, EntryBits::Dirty, 'D', f)?;
        write_bit(*self, EntryBits::Accessed, 'A', f)?;
        write_bit(*self, EntryBits::Global, 'G', f)?;
        write_bit(*self, EntryBits::UserReadable, 'U', f)?;
        write_bit(*self, EntryBits::Execute, 'X', f)?;
        write_bit(*self, EntryBits::Write, 'W', f)?;
        write_bit(*self, EntryBits::Read, 'R', f)?;
        write_bit(*self, EntryBits::Valid, 'V', f)?;
        Ok(())
    }
}

#[derive(Debug)]
pub enum EntryError {
    EntryInvalid,
    EntryIsPage,
}

impl Entry {
    pub fn is_valid(&self) -> bool {
        self.entry & EntryBits::Valid.bits() != 0
    }

    pub fn flags(&self) -> EntryBits {
        EntryBits::from_bits(self.entry & ((1 << 9) - 1)).unwrap()
    }

    /// Whether this is a leaf entry not pointing to further [`PageTable`]s.
    pub fn is_leaf(&self) -> bool {
        self.entry & EntryBits::RWX.bits() != 0
    }

    pub unsafe fn get_ptr(&self) -> *const PageTable {
        self.get_ptr_raw() as *const PageTable
    }

    pub unsafe fn get_ptr_mut(&mut self) -> *mut PageTable {
        self.get_ptr_raw() as *mut PageTable
    }

    pub unsafe fn get_ptr_raw(&self) -> usize {
        // TODO: Is this correct?
        ((self.entry << 2) & !PBIT_MASK as u64) as usize
    }

    pub unsafe fn set(&mut self, paddr: u64, flags: EntryBits) {
        self.entry = (paddr >> 2) | (flags | EntryBits::Valid).bits();
    }
}

const PBITS: usize = 12; // the page offset is 12 bits long
const PBIT_MASK: usize = (1 << PBITS) - 1;
const PPN_BITS: usize = 56;
const PADDR_MASK: usize = (1 << PPN_BITS) - 1;

// For Sv39 and Sv48, each VPN section has 9 bits in length;
const VPN_BITS: usize = 9;
const VPN_MASK: usize = (1 << VPN_BITS) - 1;

/// Convert a virtual memory address into its parts
///
/// Returns (leaf pagetable, middle pagetable, root pagetable) in this order.
const fn vpn_segments(vaddr: usize) -> [usize; 3] {
    [
        (vaddr >> (PBITS + 0 * VPN_BITS)) & VPN_MASK,
        (vaddr >> (PBITS + 1 * VPN_BITS)) & VPN_MASK,
        (vaddr >> (PBITS + 2 * VPN_BITS)) & VPN_MASK,
    ]
}

/// Map a virtual address to a physical address.
///
/// - One of Read, Write or Execute Bits must be set
/// - The paddr must be page aligned (4096 bits)
pub fn map(
    alloc: &mut BumpAllocator,
    root: &mut PageTable,
    vaddr: usize,
    paddr: usize,
    mut flags: EntryBits,
) {
    // Make sure that one of Read, Write, or Execute Bits is set.
    // Otherwise, entry is regarded as pointer to next page table level
    assert_ne!((flags & EntryBits::RWX).bits(), 0);
    // ensure that no higher bits are set then are permitted
    assert_eq!(flags.bits() & !((1 << 9) - 1), 0);

    // physical address should be at least page aligned and in PPN range
    // the assertions only allow bits 12-55 to be set
    assert_eq!(paddr & PBIT_MASK, 0);
    assert_eq!(paddr & !PADDR_MASK, 0);

    let vpn = vpn_segments(vaddr);

    // Lookup in top level page table
    let v = &mut root.entries[vpn[2]];
    if !v.is_valid() {
        alloc_missing_pagetable(v, alloc);
        assert!(v.is_valid());
    }
    let pt = unsafe { v.get_ptr_mut().as_mut().unwrap() };

    // lookup in 2nd level page
    let v = &mut pt.entries[vpn[1]];
    if !v.is_valid() {
        alloc_missing_pagetable(v, alloc);
        assert!(v.is_valid());
    }

    // Lookup in lowest level page table
    let pt = unsafe { v.get_ptr_mut().as_mut().unwrap() };
    let v = &mut pt.entries[vpn[0]];

    // TODO Assert that the entry is empty and not currently pointing at anything
    if v.is_valid() {
        log::debug!("expected invalid entry, got {v:?} {vaddr:0x}, new: {flags:?}");
        //assert!(!v.is_valid(), "remapping entry");
        flags |= v.flags();
        log::debug!("new flags {flags:?}");
    }

    // Now we are ready to point v to our physical address
    v.entry = ((paddr >> 2) | (flags | EntryBits::Valid).bits() as usize) as u64;
}

/// Allocate a missing page table if it is missing.
///
/// `entry` is an existing entry in an existing [`PageTable`] which is currently not used.
/// It will be changed to point to a newly allocated one.
fn alloc_missing_pagetable(entry: &mut Entry, alloc: &mut BumpAllocator) {
    // if the entry was valid, there's no missing PageTable
    assert!(!entry.is_valid());

    // Allocate enough space for a new PageTale
    let loc = alloc
        .alloc(mem::size_of::<PageTable>(), PAGESIZE)
        .expect("could not allocate page")
        .cast::<MaybeUninit<PageTable>>();
    let page = PageTable::init(loc);

    entry.entry = (page as u64 >> 2) | EntryBits::Valid.bits();
}

/// Convert a given virtual address to the mapped physical address
pub fn virt_to_phys(root: &PageTable, vaddr: usize) -> Option<usize> {
    let vpn = vpn_segments(vaddr);

    // lookup root table entry
    let v = &root.entries[vpn[2]];
    if !v.is_valid() {
        return None;
    }
    if v.is_leaf() {
        panic!("hugepage encountered");
    }
    let pt = unsafe { v.get_ptr().as_ref().unwrap() };

    // lookup 2nd level entry
    let v = &pt.entries[vpn[1]];
    if !v.is_valid() {
        return None;
    }
    if v.is_leaf() {
        panic!("hugepage encountered");
    }
    let pt = unsafe { v.get_ptr().as_ref().unwrap() };

    // lookup 3rd (final) entry
    let v = &pt.entries[vpn[0]];
    if !v.is_valid() {
        return None;
    }
    if !v.is_leaf() {
        panic!("non leaf page where leaf was expected");
    }

    let address = unsafe { v.get_ptr_raw() };
    return Some(address | (vaddr & PBIT_MASK));
}

/// Identity-map the address range described by `start` and `end` to the same location in virtual memory
pub fn id_map_range(
    alloc: &mut BumpAllocator,
    root: &mut PageTable,
    start: usize,
    end: usize,
    flags: EntryBits,
) {
    let ptr: *mut PageTable = (start & !(PAGESIZE - 1)) as *mut PageTable;
    let endptr: *mut PageTable = end as *mut PageTable;
    assert!(ptr <= endptr);
    log::debug!("identity-mapping from {:0x} to {:0x}", start, end);
    let mut offset = 0;
    while unsafe { ptr.add(offset) < endptr } {
        let addr = unsafe { ptr.add(offset) } as usize;
        map(alloc, root, addr, addr, flags);
        offset += 1;
    }
}

pub fn id_map_lower_huge(root: &mut PageTable) {
    let base: u64 = 1 << 30;
    for (i, entry) in root.entries[0..256].iter_mut().enumerate() {
        assert!(!entry.is_valid());
        unsafe { entry.set(base * i as u64, EntryBits::RWX | EntryBits::Valid); }
    }
}

/// Map a range of phyiscal addresses described by `start` and `size` to virtual memory starting at `virt_base`.
/// Effectively this allocates a region in virtual memory starting at `virt_base` with `size` bytes space.
pub fn map_range_alloc(
    alloc: &mut BumpAllocator,
    root: &mut PageTable,
    virt_base: usize,
    size: usize,
    flags: EntryBits,
) {
    let ptr: *mut PageTable = (virt_base & !(PAGESIZE - 1)) as *mut PageTable;
    let mut offset = 0;
    while unsafe { (ptr.add(offset) as usize) < (virt_base + size) } {
        let addr = unsafe { ptr.add(offset) } as usize;
        log::trace!("mapping page {:x}", addr);
        let page_addr = alloc
            .alloc(PAGESIZE, PAGESIZE)
            .expect("Could not alloc page")
            .cast::<u8>();
        map(alloc, root, addr, page_addr as usize, flags);
        offset += 1;
    }
}

pub unsafe fn use_pagetable(root: *mut PageTable) {
    // enable MXR (make Executable readable) bit
    // enable SUM (permit Supervisor User Memory access) bit
    unsafe {
        SStatus::set(SStatusFlags::MXR & SStatusFlags::SUM);
    }

    log::debug!("enabling new root pagetable {:p}", root);

    // Setup Root Page table in satp register
    unsafe {
        Satp::write(SatpData {
            mode: SatpMode::Sv39,
            asid: 1,
            ppn: root as u64 >> 12,
        });
    }
}
