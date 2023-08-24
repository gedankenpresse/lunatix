use allocators::{AllocInit, Allocator, Arena, ArenaAlloc, Box};
use core::alloc::Layout;
use libkernel::mem::ptrs::{MappedConstPtr, MappedMutPtr, PhysConstPtr, PhysMutPtr};

use crate::caps::KernelAlloc;
use riscv::pt;
use riscv::pt::{EntryFlags, MemoryPage, PageTable, PAGESIZE};
use riscv::PhysMapper;

pub struct KernelMapper;

unsafe impl PhysMapper for KernelMapper {
    unsafe fn phys_to_mapped_mut<T>(&self, phys: *mut T) -> *mut T {
        PhysMutPtr::from(phys).as_mapped().raw()
    }

    unsafe fn phys_to_mapped<T>(&self, phys: *const T) -> *const T {
        PhysConstPtr::from(phys).as_mapped().raw()
    }

    unsafe fn mapped_to_phys_mut<T>(&self, mapped: *mut T) -> *mut T {
        MappedMutPtr::from(mapped).as_direct().raw()
    }

    unsafe fn mapped_to_phys<T>(&self, mapped: *const T) -> *const T {
        MappedConstPtr::from(mapped).as_direct().raw()
    }
}

pub fn map(
    alloc: &KernelAlloc,
    root: &mut PageTable,
    vaddr: usize,
    paddr: usize,
    flags: EntryFlags,
) {
    while let Err(e) = riscv::pt::map(KernelMapper, root, vaddr, paddr, flags) {
        let new_pt = alloc
            .allocate(Layout::new::<MemoryPage>(), AllocInit::Zeroed)
            .unwrap()
            .as_mut_ptr()
            .cast();
        riscv::pt::map_pt(KernelMapper, root, e.level, e.target_vaddr, new_pt).unwrap();
    }
}

pub fn virt_to_phys(root: &PageTable, vaddr: usize) -> Option<usize> {
    pt::virt_to_phys(KernelMapper, root, vaddr)
}

/// Allocate a range of addresses from the given allocate and map them starting from `start_base` into the virtual
/// memory indicated by `root`.
pub fn map_range_alloc(
    alloc: &KernelAlloc,
    root: &mut PageTable,
    virt_base: usize,
    size: usize,
    flags: EntryFlags,
) {
    log::debug!(
        "allocating and mapping virtual memory: virt_base = {virt_base:0x}    size = {size:0x}"
    );
    let ptr: *mut MemoryPage = (virt_base & !(PAGESIZE - 1)) as *mut MemoryPage;
    let mut offset = 0;
    while unsafe { (ptr.add(offset) as usize) < (virt_base + size) } {
        let addr = unsafe { ptr.add(offset) } as usize;

        log::debug!("mapping page {:x}", addr);
        let page_addr = alloc
            .allocate(Layout::new::<MemoryPage>(), AllocInit::Zeroed)
            .unwrap()
            .as_ptr();

        map(
            alloc,
            root,
            addr,
            MappedConstPtr::from(page_addr).as_direct().raw() as usize,
            flags,
        );

        offset += 1;
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
