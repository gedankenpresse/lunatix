use allocators::Arena;
use bitflags::bitflags;
use core::mem::MaybeUninit;
use libkernel::arch::cpu::{SStatus, SStatusFlags, Satp, SatpData, SatpMode};
use libkernel::mem::ptrs::{MappedConstPtr, MappedMutPtr, PhysConstPtr, PhysMutPtr};
use libkernel::mem::{EntryFlags, MemoryPage, PageTable, PageTableEntry, PAGESIZE};

#[derive(Debug)]
pub enum EntryError {
    EntryInvalid,
    EntryIsPage,
}

pub trait PageTableEntryExt {
    fn get_ptr(&self) -> *const PageTable;

    fn get_ptr_mut(&mut self) -> *mut PageTable;

    unsafe fn set_pagetable(&mut self, ptr: *mut PageTable);
}

impl PageTableEntryExt for PageTableEntry {
    fn get_ptr(&self) -> *const PageTable {
        // TODO Better error handling
        PhysConstPtr::from(self.get_addr().unwrap() as *const PageTable)
            .as_mapped()
            .raw()
    }

    fn get_ptr_mut(&mut self) -> *mut PageTable {
        // TODO Better error handling
        PhysMutPtr::from(self.get_addr().unwrap() as *mut PageTable)
            .as_mapped()
            .raw()
    }

    unsafe fn set_pagetable(&mut self, ptr: *mut PageTable) {
        self.set(
            MappedMutPtr::from(ptr).as_direct().raw() as u64,
            EntryFlags::Valid,
        )
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
    alloc: &mut Arena<'static, MemoryPage>,
    root: &mut PageTable,
    vaddr: usize,
    paddr: usize,
    flags: EntryFlags,
) {
    log::debug!("[map] root: {root:p} vaddr: {vaddr:#x} paddr: {paddr:#x} flags: {flags:?}");
    // Make sure that one of Read, Write, or Execute Bits is set.
    // Otherwise, entry is regarded as pointer to next page table level
    assert_eq!(flags.bits() & EntryFlags::all().bits(), flags.bits());
    assert_ne!((flags & EntryFlags::RWX), EntryFlags::empty());

    // physical address should be at least page aligned and in PPN range
    assert!(paddr & PBIT_MASK == 0);
    assert!(paddr & !PADDR_MASK == 0);

    let vpn = vpn_segments(vaddr);

    // Helper to allocate intermediate page tables
    fn alloc_missing_page(entry: &mut PageTableEntry, alloc: &mut Arena<'static, MemoryPage>) {
        log::debug!("alloc missing: entry {entry:0p}");
        assert!(!entry.is_valid());

        // Allocate a page
        let page = alloc
            .alloc_one_raw()
            .expect("could not allocate page")
            .cast::<MaybeUninit<MemoryPage>>();
        let page = PageTable::init(page);

        unsafe {
            entry.set_pagetable(page);
        }
    }

    let mut v = &mut root.entries[vpn[2]];
    for level in (0..2).rev() {
        log::debug!("{:?}", v);
        if !v.is_valid() {
            alloc_missing_page(v, alloc);
        }
        v = &mut unsafe { v.get_ptr_mut().as_mut() }.unwrap().entries[vpn[level]];
    }

    // Now we are ready to point v to our physical address
    assert!(!v.is_valid());
    unsafe {
        v.set(paddr as u64, flags | EntryFlags::Valid);
    }
}

pub fn virt_to_phys(root: &PageTable, vaddr: usize) -> Option<usize> {
    let vpn = vpn_segments(vaddr);
    let v = &root.entries[vpn[2]];
    if !v.is_valid() {
        return None;
    }
    if v.is_leaf() {
        panic!("hugepage encountered");
    }
    let pt = unsafe { v.get_ptr().as_ref().unwrap() };
    let v = &pt.entries[vpn[1]];
    if !v.is_valid() {
        return None;
    }
    if v.is_leaf() {
        panic!("hugepage encountered");
    }
    let pt = unsafe { v.get_ptr().as_ref().unwrap() };
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

pub fn map_range_alloc(
    alloc: &mut Arena<'static, MemoryPage>,
    root: &mut PageTable,
    virt_base: usize,
    size: usize,
    flags: EntryFlags,
) {
    log::debug!("[map range alloc] virt_base {virt_base:0x} size {size:0x}");
    let ptr: *mut MemoryPage = (virt_base & !(PAGESIZE - 1)) as *mut MemoryPage;
    let mut offset = 0;
    while unsafe { (ptr.add(offset) as usize) < (virt_base + size) } {
        let addr = unsafe { ptr.add(offset) } as usize;
        log::debug!("mapping page {:x}", addr);
        let page_addr = alloc
            .alloc_one_raw()
            .expect("Could not alloc page")
            .cast::<MemoryPage>();

        map(
            alloc,
            root,
            addr,
            MappedConstPtr::from(page_addr as *const u8)
                .as_direct()
                .raw() as usize,
            flags,
        );

        offset += 1;
    }
}

pub unsafe fn use_pagetable(root: PhysMutPtr<PageTable>) {
    // enable MXR (make Executable readable) bit
    // enable SUM (premit Supervisor User Memory access) bit
    unsafe {
        SStatus::set(SStatusFlags::MXR & SStatusFlags::SUM);
    }

    log::debug!("enabling new pagetable {:p}", root);

    // Setup Root Page table in satp register
    unsafe {
        Satp::write(SatpData {
            mode: SatpMode::Sv39,
            asid: 0,
            ppn: root.raw() as u64 >> 12,
        });
    }
}

/// identity maps lower half of address space using hugepages
pub fn unmap_userspace(root: &mut PageTable) {
    for entry in root.entries[0..256].iter_mut() {
        unsafe {
            entry.clear();
        }
    }
}
