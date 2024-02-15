use allocators::AllocInit;
use allocators::{bump_allocator::BumpAllocator, Box};
use core::alloc::Layout;
use riscv::cpu::{SStatus, SStatusFlags, Satp, SatpData, SatpMode};
use riscv::mem::paddr_ppn;
use riscv::pt::{EntryFlags, MemoryPage, PageTable, PAGESIZE};
use riscv::PhysMapper;

pub struct IdMapper;

unsafe impl PhysMapper for IdMapper {
    unsafe fn phys_to_mapped_mut<T>(&self, phys: *mut T) -> *mut T {
        phys
    }

    unsafe fn phys_to_mapped<T>(&self, phys: *const T) -> *const T {
        phys
    }

    unsafe fn mapped_to_phys_mut<T>(&self, mapped: *mut T) -> *mut T {
        mapped
    }

    unsafe fn mapped_to_phys<T>(&self, mapped: *const T) -> *const T {
        mapped
    }
}

pub fn map<'a>(
    alloc: &impl BumpAllocator<'a>,
    root: &mut PageTable,
    vaddr: usize,
    paddr: usize,
    flags: EntryFlags,
) {
    while let Err(e) = riscv::pt::map(IdMapper, root, vaddr, paddr, flags) {
        let new_pt_box: Box<'_, '_, PageTable> =
            unsafe { Box::new_zeroed(alloc).unwrap().assume_init() };
        let new_pt = new_pt_box.leak();
        riscv::pt::map_pt(IdMapper, root, e.level, e.target_vaddr, new_pt).unwrap();
    }
}

pub fn virt_to_phys(root: &PageTable, vaddr: usize) -> Option<usize> {
    riscv::pt::virt_to_phys(IdMapper, root, vaddr)
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
            .allocate(Layout::new::<MemoryPage>(), AllocInit::Zeroed)
            .expect("Could not alloc page")
            .as_mut_ptr();
        map(alloc, root, addr, page_addr as usize, flags);
        offset += 1;
    }
}

pub unsafe fn use_pagetable(root: *mut PageTable) {
    assert_eq!(root as u64, paddr_ppn(root as u64));

    // enable MXR (make Executable readable) bit
    // enable SUM (permit Supervisor User Memory access) bit
    unsafe {
        SStatus::set(SStatusFlags::MXR & SStatusFlags::SUM);
    }

    log::debug!("enabling new root pagetable {:p}", root);

    // setup root page table in satp register
    unsafe {
        Satp::write(SatpData {
            mode: SatpMode::Sv39,
            asid: 1,
            ppn: paddr_ppn(root as u64),
        });
    }
}
