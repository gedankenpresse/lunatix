#[cfg(target_arch = "riscv64")]
mod riscv64;
mod userspace;
#[cfg(target_arch = "x86_64")]
mod x86_64;

use allocators::{bump_allocator::BumpAllocator, Box};
use derivation_tree::tree::DerivationTree;
use libkernel::mem::ptrs::{MappedConstPtr, PhysConstPtr, PhysMutPtr};

use riscv::pt::PageTable;
#[cfg(target_arch = "riscv64")]
pub use riscv64::*;
#[cfg(target_arch = "x86_64")]
pub use x86_64::*;

use crate::{
    arch_specific::plic::PLIC,
    caps::{Capability, KernelAlloc},
    InitCaps, KERNEL_ALLOCATOR, KERNEL_ROOT_PT,
};
pub use userspace::{create_init_caps, load_init_binary, map_device_tree};

/// Create an allocator that can be used for kernel initialization
pub fn init_alloc(phys_mem_start: PhysMutPtr<u8>, phys_mem_end: PhysMutPtr<u8>) -> KernelAlloc {
    log::debug!("start: {phys_mem_start:?}, end: {phys_mem_end:?}");
    let virt_start = phys_mem_start.as_mapped().raw();
    let virt_end = phys_mem_end.as_mapped().raw();
    log::debug!("virt_start: {virt_start:p} virt_end: {virt_end:p}");
    let mem_slice: &mut [u8] = unsafe {
        core::slice::from_raw_parts_mut(
            virt_start.cast::<u8>(),
            virt_end as usize - virt_start as usize,
        )
    };

    log::debug!("Init Kernel Allocator");
    KernelAlloc::new(mem_slice)
}

pub fn init_kernel_allocator(
    phys_mem_start: PhysMutPtr<u8>,
    phys_mem_end: PhysMutPtr<u8>,
) -> &'static KernelAlloc {
    unsafe { KERNEL_ALLOCATOR = Some(init_alloc(phys_mem_start, phys_mem_end)) };
    let allocator: &'static KernelAlloc = unsafe { (&mut KERNEL_ALLOCATOR).as_mut().unwrap() };
    allocator
}

pub fn init_device_tree(dtb: *const u8) -> fdt_rs::base::DevTree<'static> {
    //parse device tree from bootloader
    return unsafe { fdt_rs::base::DevTree::from_raw_pointer(dtb).unwrap() };
}

pub fn init_plic() -> &'static mut PLIC {
    let plic = unsafe {
        use crate::arch_specific::plic::*;
        let plic = init_plic(PhysMutPtr::from(0xc000000 as *mut PLIC).as_mapped().raw());
        plic
    };

    // TODO: determine which context(s) we should enable
    plic.set_threshold(1, 0);
    plic.set_threshold(1, 1);
    plic.set_threshold(1, 2);
    plic.set_threshold(1, 3);
    plic
}

pub fn init_kernel_root_pt() {
    let kernel_root_pt = init_kernel_pagetable();
    unsafe { KERNEL_ROOT_PT = MappedConstPtr::from(kernel_root_pt as *const PageTable).as_direct() }
}

pub fn init_derivation_tree<'a>(
    allocator: &'a KernelAlloc,
) -> Box<'a, 'static, DerivationTree<Capability>> {
    // fill the derivation tree with initially required capabilities
    let mut derivation_tree = Box::new_uninit(allocator).unwrap();
    let derivation_tree = unsafe {
        DerivationTree::init_with_root_value(&mut derivation_tree, Capability::empty());
        derivation_tree.assume_init()
    };
    return derivation_tree;
}

pub fn load_init_task(
    derivation_tree: &DerivationTree<Capability>,
    init_caps: &mut InitCaps,
    dtb: PhysConstPtr<u8>,
) {
    // load the init binary
    {
        let mut mem_cap = derivation_tree.get_root_cursor().unwrap();
        let mut mem_cap = mem_cap.get_exclusive().unwrap();
        load_init_binary(&mut init_caps.init_task, &mut mem_cap);
    }
}

pub fn prepare_userspace_handoff() {
    log::debug!("enabling interrupts");
    riscv::timer::set_next_timer(0).unwrap();
    riscv::trap::enable_interrupts();

    unsafe { set_return_to_user() };
}
