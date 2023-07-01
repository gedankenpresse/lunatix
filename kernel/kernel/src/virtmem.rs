use bitflags::bitflags;
use core::mem::MaybeUninit;
use memory::Arena;

use crate::mem::{Page, PAGESIZE, self};

#[derive(Copy, Clone)]
pub struct Entry {
    entry: u64,
}

pub struct PageTable {
    pub entries: [Entry; 512],
}

impl core::fmt::Debug for Entry {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Entry")
            .field("ppn", unsafe { &self.get_ptr() })
            .field("flags", &self.flags())
            .finish()
    }
}

impl PageTable {
    pub fn empty(alloc: &mut Arena<'static, Page>) -> Option<*mut PageTable> {
        let page = alloc.alloc_one_raw()?;
        unsafe {
            for i in 0..PAGESIZE {
                *page.cast::<u8>().add(i) = 0;
            }
        }
        Some(page.cast::<PageTable>())
    }

    pub fn init(page: *mut MaybeUninit<Page>) -> *mut PageTable {
        unsafe {
            for i in 0..PAGESIZE {
                *page.cast::<u8>().add(i) = 0;
            }
        }
        page.cast::<PageTable>()
    }

    // This doesn't do a deep copy, so it should only be used for global mappings
    pub fn init_copy(page: *mut MaybeUninit<Page>, orig: &PageTable) -> *mut PageTable {
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
}

impl PageTable {
    pub fn len() -> usize {
        return 512;
    }
}

bitflags! {
    #[derive(Debug)]
    pub struct EntryBits: u64 {
        const Valid = 1 << 0;
        const Read = 1 << 1;
        const Write = 1 << 2;
        const Execute = 1 << 3;
        const UserReadable = 1 << 4;
        const Global = 1 << 5;
        const Accessed = 1 << 6;
        const Dirty = 1 << 7;

        const RWX = Self::Read.bits() | Self::Write.bits() | Self::Execute.bits();
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

    pub fn is_invalid(&self) -> bool {
        !self.is_valid()
    }
    pub fn is_leaf(&self) -> bool {
        self.entry & EntryBits::RWX.bits() != 0
    }

    pub fn flags(&self) -> EntryBits {
        EntryBits::from_bits_retain(self.entry & EntryBits::all().bits())
    }

    pub unsafe fn get_ptr(&self) -> *const PageTable {
        mem::phys_to_kernel_usize(self.get_phys_usize()) as *const PageTable
    }

    pub unsafe fn get_ptr_mut(&mut self) -> *mut PageTable {
        mem::phys_to_kernel_usize(self.get_phys_usize()) as *mut PageTable
    }

    pub unsafe fn get_phys_usize(&self) -> usize {
        // TODO: Is this correct?
        let phys = ((self.entry << 2) & !((1 << 12) - 1)) as usize;
        return phys;
    }

    pub fn get_pagetable_mut(&mut self) -> Result<&mut PageTable, EntryError> {
        if self.is_invalid() {
            return Err(EntryError::EntryInvalid);
        }
        if self.is_leaf() {
            return Err(EntryError::EntryIsPage);
        }

        return Ok(unsafe { self.get_ptr_mut() .as_mut() }.unwrap());
    }


    pub unsafe fn set(&mut self, paddr: u64, flags: EntryBits) {
        self.entry = (paddr >> 2) | flags.bits();
    }

    pub unsafe fn set_pagetable(&mut self, pt: *mut PageTable) {
         self.set(mem::kernel_to_phys_mut_ptr(pt).0 as u64, EntryBits::Valid);
    }
}

const PBITS: usize = 12; // the page offset is 12 bits long
const PBIT_MASK: usize = (1 << PBITS) - 1;
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

pub fn map(
    alloc: &mut memory::Arena<'static, Page>,
    root: &mut PageTable,
    vaddr: usize,
    paddr: usize,
    bits: usize,
) {
    log::debug!("[map] root: {root:p} vaddr: {vaddr:0x} paddr: {paddr:0x} bits: {bits:?}");
    // Make sure that one of Read, Write, or Execute Bits is set.
    // Otherwise, entry is regarded as pointer to next page table level
    assert!(bits & EntryBits::RWX.bits() as usize != 0);
    assert!(bits & !((1 << 10) - 1) == 0);

    // physical address should be at least page aligned and in PPN range
    assert!(paddr & PBIT_MASK == 0);
    assert!(paddr & !PADDR_MASK == 0);

    let vpn = vpn_segments(vaddr);

    // Helper to allocate intermediate page tables
    fn alloc_missing_page(entry: &mut Entry, alloc: &mut Arena<'static, Page>) {
        log::debug!("alloc missing: entry {entry:0p}");
        assert!(entry.is_invalid());

        // Allocate a page
        let page = alloc
            .alloc_one_raw()
            .expect("could not allocate page")
            .cast::<MaybeUninit<Page>>();
        let page = PageTable::init(page);

        unsafe { entry.set_pagetable(page); }
    }

    // Lookup in top level page table
    let v = &mut root.entries[vpn[2]];
    if !v.is_valid() {
        alloc_missing_page(v, alloc);
    }
    let pt = v.get_pagetable_mut().unwrap();
    let v = &mut pt.entries[vpn[1]];
    if !v.is_valid() {
        alloc_missing_page(v, alloc);
    }

    // Lookup in lowest level page table
    let pt = v.get_pagetable_mut().unwrap();
    let v = &mut pt.entries[vpn[0]];

    // Now we are ready to point v to our physical address
    v.entry = ((paddr >> 2) | bits | EntryBits::Valid.bits() as usize) as u64;
}

pub fn virt_to_phys(root: &PageTable, vaddr: usize) -> Option<usize> {
    let vpn = vpn_segments(vaddr);
    let v = &root.entries[vpn[2]];
    if v.is_invalid() {
        return None;
    }
    if v.is_leaf() {
        panic!("hugepage encountered");
    }
    let pt = unsafe { v.get_ptr().as_ref().unwrap() };
    let v = &pt.entries[vpn[1]];
    if v.is_invalid() {
        return None;
    }
    if v.is_leaf() {
        panic!("hugepage encountered");
    }
    let pt = unsafe { v.get_ptr().as_ref().unwrap() };
    let v = &pt.entries[vpn[0]];
    if v.is_invalid() {
        return None;
    }
    if !v.is_leaf() {
        panic!("non leaf page where leaf was expected");
    }

    let address = unsafe { v.get_phys_usize() };
    return Some(address | (vaddr & PBIT_MASK));
}

pub fn id_map_range(
    alloc: &mut Arena<'static, Page>,
    root: &mut PageTable,
    start: usize,
    end: usize,
    bits: usize,
) {
    let ptr: *mut Page = (start & !(PAGESIZE - 1)) as *mut Page;
    let endptr: *mut Page = end as *mut Page;
    assert!(ptr <= endptr);
    log::debug!("[id_map] start {:0x} end {:0x}", start, end);
    let mut offset = 0;
    while unsafe { ptr.add(offset) < endptr } {
        let addr = unsafe { ptr.add(offset) } as usize;
        map(alloc, root, addr, addr, bits);
        offset += 1;
    }
}

pub fn map_range_alloc(
    alloc: &mut Arena<'static, Page>,
    root: &mut PageTable,
    virt_base: usize,
    size: usize,
    bits: usize,
) {
    log::debug!("[map range alloc] virt_base {virt_base:0x} size {size:0x}");
    let ptr: *mut Page = (virt_base & !(PAGESIZE - 1)) as *mut Page;
    let mut offset = 0;
    while unsafe { (ptr.add(offset) as usize) < (virt_base + size) } {
        let addr = unsafe { ptr.add(offset) } as usize;
        log::debug!("mapping page {:x}", addr);
        let page_addr = alloc
            .alloc_one_raw()
            .expect("Could not alloc page")
            .cast::<Page>();
        map(alloc, root, addr, mem::kernel_to_phys_ptr(page_addr).0 as usize, bits);
        offset += 1;
    }
}

pub fn create_kernel_page_table(
    allocator: &mut Arena<'static, Page>,
    mem_start: usize,
    mem_length: usize,
) -> Result<*mut PageTable, ()> {
    let root = PageTable::empty(allocator).unwrap();
    let root_ref = unsafe { root.as_mut().unwrap() };
    let rwx = EntryBits::RWX.bits() as usize;
    // Map Kernel Memory
    id_map_range(allocator, root_ref, mem_start, mem_start + mem_length, rwx);
    // Map Uart
    id_map_range(allocator, root_ref, 0x1000_0000, 0x1000_0000 + 0x1000, rwx);
    // Map Shutdown
    id_map_range(allocator, root_ref, 0x100_000, 0x100_000 + 0x1000, rwx);
    return Ok(root);
}

pub unsafe fn use_pagetable(root: mem::PhysMutPtr<PageTable>) {
    use crate::arch::cpu::*;

    // enable MXR (make Executable readable) bit
    // enable SUM (premit Supervisor User Memory access) bit
    unsafe {
        SStatus::set(SStatusFlags::MXR & SStatusFlags::SUM);
    }

    log::debug!("enabling new pagetable {:p}", root.0);

    // Setup Root Page table in satp register
    unsafe {
        Satp::write(SatpData {
            mode: SatpMode::Sv39,
            asid: 0,
            ppn: root.0 as u64 >> 12,
        });
    }
}


/// identity maps lower half of address space using hugepages
pub fn unmap_userspace(root: &mut PageTable) {
    for entry in root.entries[0..256].iter_mut() {
        unsafe { entry.set(0, EntryBits::empty()); }
    }
}