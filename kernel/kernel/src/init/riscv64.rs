use allocators::{AllocInit, Allocator, Arena, ArenaAlloc};
use core::alloc::Layout;
use core::ops::{Deref, DerefMut};
use libkernel::mem::ptrs::{MappedMutPtr, PhysMutPtr};
use riscv::cpu;
use riscv::pt::{MemoryPage, PageTable};
use riscv::trap::{trap_frame_restore, TrapFrame};

use crate::caps::task::TaskState;
use crate::caps::KernelAlloc;
use crate::{caps, mmu, virtmem, INIT_CAPS};

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

/// Allocated enough space for the stack of the kernel trap handler and return a pointer to the start of it.
///
/// The stack is allocated from the given allocator and holds the specified number of memory pages.
pub fn alloc_trap_handler_stack(allocator: &KernelAlloc, num_pages: usize) -> *mut () {
    let stack = allocator
        .allocate(
            Layout::array::<MemoryPage>(num_pages).unwrap(),
            AllocInit::Zeroed,
        )
        .unwrap();
    let stack_end = stack.as_mut_ptr().cast::<MemoryPage>();
    let stack_start = unsafe { stack_end.add(num_pages) as *mut () };

    log::debug!("allocated trap handler stack: {stack_start:p} - {stack_end:p}");
    return stack_start;
}

/// Allocate a [`TrapFrame`] from the given allocator, assign the given trap handler stack to it and configure
/// [`SScratch`](cpu::SScratch) to point to it.
pub fn init_kernel_trap_handler(allocator: &KernelAlloc, trap_stack_start: *mut ()) {
    let trap_frame: *mut TrapFrame = allocator
        .allocate(Layout::new::<TrapFrame>(), AllocInit::Uninitialized)
        .unwrap()
        .as_mut_ptr()
        .cast();

    unsafe {
        (*trap_frame).trap_handler_stack = trap_stack_start as *mut usize;
        cpu::SScratch::write(trap_frame as usize);
    }

    log::debug!("initialized kernel trap frame at {trap_frame:p}");
}

/// Yield to the task that owns the given `trap_frame`
unsafe fn yield_to_task(trap_handler_stack: *mut u8, task: &mut caps::Capability) -> ! {
    let mut task = task.get_task_mut().unwrap();
    let task = task.as_mut();
    unsafe {
        crate::sched::set_active_task(task.state.borrow_mut().deref_mut() as *mut TaskState);
    }
    let mut state = unsafe { task.state.borrow_mut() };
    state.frame.trap_handler_stack = trap_handler_stack.cast();

    let mut vspace = state.vspace.get_vspace_mut().unwrap();
    let vspace = vspace.as_mut();
    log::debug!("enabling task pagetable");
    unsafe {
        mmu::use_pagetable(MappedMutPtr::from(vspace.root).as_direct());
    }
    log::debug!("restoring trap frame");
    trap_frame_restore(&mut state.frame as *mut TrapFrame);
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
