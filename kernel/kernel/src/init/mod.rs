#[cfg(target_arch = "riscv64")]
mod riscv64;
mod userspace;
#[cfg(target_arch = "x86_64")]
mod x86_64;

use allocators::Arena;
use libkernel::mem::{ptrs::PhysMutPtr, MemoryPage, PAGESIZE};

#[cfg(target_arch = "riscv64")]
pub(crate) use riscv64::*;
#[cfg(target_arch = "x86_64")]
pub(crate) use x86_64::*;

pub(crate) use userspace::*;

pub(crate) fn init_alloc(
    phys_mem_start: PhysMutPtr<u8>,
    phys_mem_end: PhysMutPtr<u8>,
) -> Arena<'static, MemoryPage> {
    log::debug!("start: {phys_mem_start:?}, end: {phys_mem_end:?}");
    let virt_start = phys_mem_start.as_mapped().raw();
    let virt_end = phys_mem_end.as_mapped().raw();
    log::debug!("virt_start: {virt_start:p} virt_end: {virt_end:p}");
    let mem_slice: &mut [MemoryPage] = unsafe {
        core::slice::from_raw_parts_mut(
            virt_start.cast::<MemoryPage>(),
            (virt_end as usize - virt_start as usize) / PAGESIZE,
        )
    };

    log::debug!("Init Kernel Allocator");
    let allocator = Arena::new(mem_slice);
    return allocator;
}
