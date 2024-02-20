use allocators::AllocInit;
use allocators::{bump_allocator::BumpAllocator, Box};
use bitflags::Flags;
use core::alloc::Layout;
use core::mem::MaybeUninit;
use riscv::cpu::{SStatus, SStatusFlags, Satp, SatpData, SatpMode};
use riscv::mem::mapping::{PageType, PhysMapping};
use riscv::mem::paddr::PAddr;
use riscv::mem::vaddr::VAddr;
use riscv::mem::{paddr, vaddr, EntryFlags, MemoryPage, PageTable, PAGESIZE};
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

pub fn virt_to_phys(root: &PageTable, vaddr: usize) -> Option<usize> {
    riscv::pt::virt_to_phys(IdMapper, root, vaddr)
}

/// Identity-map the address range described by `start` and `end` to the same location in virtual memory
pub fn id_map_range<'a>(
    alloc: &mut impl BumpAllocator<'a>,
    root: &mut PageTable,
    phy_map: &PhysMapping,
    start: u64,
    end: u64,
    flags: EntryFlags,
) {
    let ptr: *mut PageTable = (start & !(PAGESIZE as u64 - 1)) as *mut PageTable;
    let endptr: *mut PageTable = end as *mut PageTable;
    assert!(ptr <= endptr);
    log::debug!("identity-mapping from {:0x} to {:0x}", start, end);
    let mut offset = 0;
    while unsafe { ptr.add(offset) < endptr } {
        let addr = unsafe { ptr.add(offset) } as u64;
        riscv::mem::mapping::map(alloc, root, phy_map, addr, addr, flags, PageType::Page);
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
pub fn kernel_map_phys_huge(root: &mut PageTable) -> PhysMapping {
    const GB: u64 = 1024 * 1024 * 1024;
    const mapping: PhysMapping = PhysMapping::new(0, 8 * GB);
    for (i, entry) in root.entries[256..256 + 64].iter_mut().enumerate() {
        assert!(!entry.is_valid());
        unsafe {
            entry.set(
                i as u64 * GB,
                EntryFlags::Accessed | EntryFlags::Dirty | EntryFlags::RWX | EntryFlags::Valid,
            );
        }
    }

    return mapping;
}

/// Allocate a region in virtual memory starting at `virt_base` with `size` bytes space.
///
/// All necessary intermediate pagetables are automatically allocated from the given allocator.
pub fn map_range_alloc<'a>(
    alloc: &impl BumpAllocator<'a>,
    root: &mut PageTable,
    phy_map: &PhysMapping,
    virt_base: VAddr,
    size: u64,
    flags: EntryFlags,
) {
    log::trace!(
        "configuring multiple address translation mappings for range of {:#x} bytes from {:#x} to {:#x}",
        size,
        virt_base,
        virt_base + size,
    );
    assert_eq!(
        virt_base & !(PAGESIZE as u64 - 1),
        virt_base,
        "virt_base {:#x} is not aligned to page boundaries, cannot map",
        virt_base
    );

    let ptr: *mut PageTable = virt_base as *mut PageTable;
    let mut offset = 0;
    while unsafe { (ptr.add(offset) as u64) < (virt_base + size) } {
        let addr = unsafe { ptr.add(offset) } as u64;
        let page_addr = alloc
            .allocate(Layout::new::<MemoryPage>(), AllocInit::Zeroed)
            .expect("Could not alloc page for new intermediate PageTable")
            .as_mut_ptr();
        riscv::mem::mapping::map(
            alloc,
            root,
            phy_map,
            addr,
            page_addr as u64,
            flags,
            PageType::Page,
        );
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

    // setup root page table in satp register
    unsafe {
        Satp::write(SatpData::new(SatpMode::Sv39, 1, root as PAddr));
    }
}
