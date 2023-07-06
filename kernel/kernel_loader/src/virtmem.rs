use allocators::bump_allocator::BumpAllocator;
use allocators::AllocInit;
use bitflags::{bitflags, Flags};
use core::fmt::Write;
use core::mem;
use core::mem::MaybeUninit;
use libkernel::arch::cpu::{SStatus, SStatusFlags, Satp, SatpData, SatpMode};
use libkernel::mem::{EntryFlags, MemoryPage, PageTable, PageTableEntry, PAGESIZE};

pub trait PageTableEntryExt {
    unsafe fn get_ptr(&self) -> *const PageTable;

    unsafe fn get_ptr_mut(&mut self) -> *mut PageTable;
}

impl PageTableEntryExt for PageTableEntry {
    unsafe fn get_ptr(&self) -> *const PageTable {
        self.get_addr().unwrap() as *const PageTable // TODO Better error handling
    }

    unsafe fn get_ptr_mut(&mut self) -> *mut PageTable {
        self.get_addr().unwrap() as *mut PageTable // TODO Better error handling
    }
}

#[derive(Debug)]
pub enum EntryError {
    EntryInvalid,
    EntryIsPage,
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
pub fn map<'a>(
    alloc: &impl BumpAllocator<'a>,
    root: &mut PageTable,
    vaddr: usize,
    paddr: usize,
    mut flags: EntryFlags,
) {
    // Make sure that one of Read, Write, or Execute Bits is set.
    // Otherwise, entry is regarded as pointer to next page table level
    assert_ne!((flags & EntryFlags::RWX).bits(), 0);
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
        flags |= v.get_flags();
        log::debug!("new flags {flags:?}");
    }

    // Now we are ready to point v to our physical address
    unsafe { v.set(paddr as u64, flags | EntryFlags::Valid) }
}

/// Allocate a missing page table if it is missing.
///
/// `entry` is an existing entry in an existing [`PageTable`] which is currently not used.
/// It will be changed to point to a newly allocated one.
fn alloc_missing_pagetable<'a>(entry: &mut PageTableEntry, alloc: &impl BumpAllocator<'a>) {
    // if the entry was already valid, there's no missing PageTable
    assert!(!entry.is_valid());

    // Allocate enough space for a new PageTale
    let new_pagetable = PageTable::new(alloc).expect("Could not allocate missing PageTable");
    unsafe { entry.set(new_pagetable.into_raw() as u64, EntryFlags::Valid) };
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

    let address = v.get_addr().unwrap();
    Some(address | (vaddr & PBIT_MASK))
}

/// Identity-map the address range described by `start` and `end` to the same location in virtual memory
pub fn id_map_range<'a>(
    alloc: &mut impl BumpAllocator<'a>,
    root: &mut PageTable,
    start: usize,
    end: usize,
    flags: EntryFlags,
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

/// identity maps lower half of address space using hugepages
pub fn id_map_lower_huge(root: &mut PageTable) {
    let base: u64 = 1 << 30;
    for (i, entry) in root.entries[0..256].iter_mut().enumerate() {
        assert!(!entry.is_valid());
        unsafe {
            entry.set(
                base * i as u64,
                EntryFlags::Accessed | EntryFlags::Dirty | EntryFlags::RWX | EntryFlags::Valid,
            );
        }
    }
}

/// maps physical memory into lower half of kernel memory
pub fn kernel_map_phys_huge(root: &mut PageTable) {
    const GB: u64 = 1024 * 1024 * 1024;
    for (i, entry) in root.entries[256..256 + 64].iter_mut().enumerate() {
        assert!(!entry.is_valid());
        unsafe {
            entry.set(
                i as u64 * GB,
                EntryFlags::Accessed | EntryFlags::Dirty | EntryFlags::RWX | EntryFlags::Valid,
            );
        }
    }
}

/// Map a range of phyiscal addresses described by `start` and `size` to virtual memory starting at `virt_base`.
/// Effectively this allocates a region in virtual memory starting at `virt_base` with `size` bytes space.
pub fn map_range_alloc<'a>(
    alloc: &impl BumpAllocator<'a>,
    root: &mut PageTable,
    virt_base: usize,
    size: usize,
    flags: EntryFlags,
) {
    let ptr: *mut PageTable = (virt_base & !(PAGESIZE - 1)) as *mut PageTable;
    let mut offset = 0;
    while unsafe { (ptr.add(offset) as usize) < (virt_base + size) } {
        let addr = unsafe { ptr.add(offset) } as usize;
        log::trace!("mapping page {:x}", addr);
        let page_addr = alloc
            .allocate(
                mem::size_of::<MemoryPage>(),
                mem::align_of::<MemoryPage>(),
                AllocInit::Zeroed,
            )
            .expect("Could not alloc page")
            .as_mut_ptr();
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
