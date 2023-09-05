use libkernel::mem::ptrs::{MappedMutPtr, PhysMutPtr};
use riscv::cpu;
use riscv::pt::PageTable;
use riscv::trap::{trap_frame_load, TrapFrame, TrapInfo};

use crate::{caps, mmu, virtmem};

/// Initialize the currently active PageTable with virtual address mapping that is appropriate for kernel usage only.
///
/// In detail, this function reads the address of the currently active PageTable from [`Satp`](cpu::Satp), ensures
/// that the userspace area of the pagetable is unmapped and that the kernel area is correctly mapped.
pub fn init_kernel_pagetable() -> &'static mut PageTable {
    // clean up userspace mapping from kernel loader
    log::debug!("Cleaning up userspace mapping from kernel loader");
    let root_pagetable_phys = (cpu::Satp::read().ppn << 12) as *mut PageTable;
    log::debug!("Kernel Pagetable Phys: {root_pagetable_phys:p}");
    let root_pt = unsafe {
        PhysMutPtr::from(root_pagetable_phys)
            .as_mapped()
            .raw()
            .as_mut()
            .unwrap()
    };
    virtmem::unmap_userspace(root_pt);
    unsafe {
        core::arch::asm!("sfence.vma");
    }
    root_pt
}

pub fn prepare_task(task: &mut caps::Capability) {
    let mut task = task.get_task_mut().unwrap();
    let task = task.as_mut();
    let mut state = task.state.borrow_mut();
    let mut vspace = state.vspace.get_vspace_mut().unwrap();
    let vspace = vspace.as_mut();
    log::trace!("enabling task pagetable");
    unsafe {
        mmu::use_pagetable(MappedMutPtr::from(vspace.root).as_direct());
    }
}

/// Yield to the task that owns the given `trap_frame`
#[must_use]
pub fn yield_to_task(task: &mut caps::Capability) -> TrapInfo {
    let mut task = task.get_task_mut().unwrap();
    let task = task.as_mut();
    let mut state = task.state.borrow_mut();
    log::trace!("loading trap frame");
    unsafe { trap_frame_load(&mut state.frame as *mut TrapFrame) };
    log::trace!("returned from trap handler");
    TrapInfo::from_current_regs()
}

pub unsafe fn set_return_to_user() {
    log::debug!("clearing sstatus.SPP flag to enable returning to user code");
    cpu::SStatus::clear(cpu::SStatusFlags::SPP);
}
