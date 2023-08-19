#[cfg(target_arch = "riscv64")]
mod riscv64;
mod userspace;
#[cfg(target_arch = "x86_64")]
mod x86_64;

use allocators::bump_allocator::BumpAllocator;
use libkernel::mem::ptrs::PhysMutPtr;

#[cfg(target_arch = "riscv64")]
pub use riscv64::*;
#[cfg(target_arch = "x86_64")]
pub use x86_64::*;

use crate::caps::KernelAlloc;
pub use userspace::create_init_caps;

/// Create an allocator that can be used for kernel initialization
pub fn init_alloc(phys_mem_start: PhysMutPtr<u8>, phys_mem_end: PhysMutPtr<u8>) -> KernelAlloc {
    log::debug!("start: {phys_mem_start:?}, end: {phys_mem_end:?}");
    let virt_start = phys_mem_start.as_mapped().raw();
    let virt_end = phys_mem_end.as_mapped().raw();
    log::debug!("virt_start: {virt_start:p} virt_end: {virt_end:p}");
    let mem_slice: &mut [u8] = unsafe {
        core::slice::from_raw_parts_mut(
            virt_start.cast::<u8>(),
            (virt_end as usize - virt_start as usize),
        )
    };

    log::debug!("Init Kernel Allocator");
    KernelAlloc::new(mem_slice)
}
