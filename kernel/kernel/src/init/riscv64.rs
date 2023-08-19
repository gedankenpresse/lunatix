use allocators::{Arena, ArenaAlloc};
use libkernel::mem::ptrs::{MappedMutPtr, PhysMutPtr};
use riscv::cpu;
use riscv::pt::{MemoryPage, PageTable};
use riscv::trap::{trap_frame_restore, TrapFrame};

use crate::{caps, mmu, virtmem, INIT_CAPS};

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

pub fn init_trap_handler_stack(allocator: &mut Arena<'static, MemoryPage>) -> *mut () {
    let trap_handler_stack: *mut MemoryPage = unsafe { allocator.alloc_many(10).cast() };
    let stack_start = unsafe { trap_handler_stack.add(10) as *mut () };
    log::debug!("trap_stack: {stack_start:p}");
    return stack_start;
}

pub fn init_kernel_trap_handler(
    allocator: &mut Arena<'static, MemoryPage>,
    trap_stack_start: *mut (),
) {
    let trap_frame: *mut TrapFrame = unsafe { allocator.alloc_one().cast() };
    unsafe { (*trap_frame).trap_handler_stack = trap_stack_start as *mut usize };
    unsafe {
        cpu::SScratch::write(trap_frame as usize);
    }
    log::debug!("trap frame: {trap_frame:p}");
}

/// Yield to the task that owns the given `trap_frame`
unsafe fn yield_to_task(trap_handler_stack: *mut u8, task: &mut caps::Capability) -> ! {
    let taskref = task.get_task_mut().unwrap().as_mut();
    unsafe {
        crate::sched::set_active_task(taskref.state);
    }
    let state = unsafe { taskref.state.as_mut().unwrap() };
    let trap_frame = &mut state.frame;
    trap_frame.trap_handler_stack = trap_handler_stack.cast();
    let vspace = state.vspace.get_vspace_mut().unwrap().as_mut();
    log::debug!("enabling task pagetable");
    unsafe {
        mmu::use_pagetable(MappedMutPtr::from(vspace.root).as_direct());
    }
    log::debug!("restoring trap frame");
    trap_frame_restore(trap_frame as *mut TrapFrame);
}

pub fn run_init(trap_stack: *mut ()) {
    unsafe {
        set_return_to_user();
        let mut guard = INIT_CAPS.try_lock().unwrap();
        let mut task = &mut guard.init_task;
        yield_to_task(trap_stack as *mut u8, &mut task);
    };
}

unsafe fn set_return_to_user() {
    log::debug!("clearing sstatus.SPP flag to enable returning to user code");
    cpu::SStatus::clear(cpu::SStatusFlags::SPP);
}
