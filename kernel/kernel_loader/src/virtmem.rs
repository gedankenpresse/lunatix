//! Implementation of all virtual memory configuration
//!
//! # Virtual Address Regions
//!
//! This kernel is currently hardcoded for RiscV Sv39 virtual addressing using the following
//! memory regions:
//!
//! | VAddr Start | VAddr End | Size | Usage |
//! | :---------- | :-------- | :--: | ----- |
//! | | | | **Per user context virtual memory** |
//! | `0x0000000000000000` | `0x0000003fffffffff` | 256 GB | userspace virtual memory
//! | | | | **Misc** |
//! | `0x0000004000000000` | `0xFFFFFFBFFFFFFFFF` | ~16M TB | unusable addresses
//! | | | | **Kernel-space virtual memory. Shared between all user contexts** |
//! | `0xFFFFFFC000000000` | `0xFFFFFFCFFFFFFFFF` | 64 GB | direct mapping of all physical memory
//! | ... | ... | ... | currently unused
//! | `0xFFFFFFFF00000000` | `0xFFFFFFFFFFFFFFFF` | 4 GB | Kernel
//!
//! ## Reasoning
//!
//! The above split between memory regions were chosen because:
//!
//! - The RiscV spec requires the virtual address bits 63-39 be equal to bit 38.
//!   This results in the large chunk of unusable addresses.
//! - The kernel regularly requires accessing physical addresses.
//!   To avoid switching virtual addressing on and off in these cases, the physical memory
//!   is directly mapped to virtual addresses.
//!   Since this is done by the kernel, translating physical to kernel-mapped addresses is easy.
//! - Because the kernel is being executed while virtual addressing is turned on, its code, data and other ELF content
//!   needs to be available through virtual addresses.
//!   For this, the kernel ELF binary is placed at the very last usable addresses.
//!

use allocators::{bump_allocator::BumpAllocator, Box};
use allocators::{AllocInit, Allocator};
use bitflags::Flags;
use core::alloc::Layout;
use core::mem::MaybeUninit;
use riscv::cpu::{SStatus, SStatusFlags, Satp, SatpData, SatpMode};
use riscv::mem::mapping::{PageType, PhysMapping};
use riscv::mem::paddr::PAddr;
use riscv::mem::vaddr::VAddr;
use riscv::mem::{paddr, vaddr, EntryFlags, MemoryPage, PageTable, PAGESIZE};
use riscv::PhysMapper;

/// The virtual memory address at which userspace tasks are mapped
///
/// See the [module documentation](super::mem) for an explanation of this value.
pub const VIRT_MEM_USER_START: usize = 0x0;

/// The last virtual memory address at which userspace tasks are mapped.
///
/// See the [module documentation](super::mem) for an explanation of this value.
pub const VIRT_MEM_USER_END: usize = 0x0000003fffffffff;

/// The virtual memory address at which physical memory starts being mapped.
///
/// See the [module documentation](super::mem) for an explanation of this value.
pub const VIRT_MEM_PHYS_MAP_START: usize = 0xFFFFFFC000000000;

/// The last virtual memory address at which physical memory is mapped.
///
/// See the [module documentation](super::mem) for an explanation of this value.
pub const VIRT_MEM_PHYS_MAP_END: usize = 0xFFFFFFCFFFFFFFFF;

/// The virtual memory address at which the kernel binary is mapped and where the kernel stack is located
///
/// See the [module documentation](super::mem) for an explanation of this value.
pub const VIRT_MEM_KERNEL_START: usize = 0xFFFFFFFF00000000;

/// The virtual memory address at which the kernel memory ends.
///
/// See the [module documentation](super::mem) for an explanation of this value.
pub const VIRT_MEM_KERNEL_END: usize = 0xFFFFFFFFFFFFFFFF;

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

#[deprecated]
pub fn virt_to_phys(root: &PageTable, vaddr: usize) -> Option<usize> {
    riscv::pt::virt_to_phys(IdMapper, root, vaddr)
}

/// Setup virtual memory mapping of the physical memory region, returning a `PhysMapping` instance which describes the
/// mapping that was set up.
///
/// The mapping is set up using _GigaPages_ so no intermediate pagetables are allocated.
/// The passed allocator is only needed because of an underlying function signature.
pub fn setup_phys_mapping<'a>(
    page_table: &mut PageTable,
    alloc: &impl Allocator<'a>,
) -> PhysMapping {
    const MAPPING: PhysMapping = PhysMapping::new(
        VIRT_MEM_PHYS_MAP_START as u64,
        (VIRT_MEM_PHYS_MAP_END - VIRT_MEM_PHYS_MAP_START) as u64,
    );

    for i in (0..MAPPING.size).step_by(PageType::GigaPage.size() as usize) {
        riscv::mem::mapping::map(
            alloc,
            page_table,
            &PhysMapping::identity(),
            MAPPING.map(i),
            i,
            EntryFlags::RWX | EntryFlags::Accessed | EntryFlags::Dirty,
            PageType::GigaPage,
        );
    }

    return MAPPING;
}

pub fn setup_lower_mem_id_map() {
    todo!()
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
#[deprecated]
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

/// Configure the hardware to enable virtual memory using the given page table as root table
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
