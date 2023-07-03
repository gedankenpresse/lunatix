#![no_std]
#![no_main]

mod caps;
mod init;
mod mem;
mod printk;
mod virtmem;

use crate::caps::CSlot;
use crate::mem::kernel_to_phys_mut_ptr;
use crate::mem::{phys_to_kernel_mut_ptr, Page, PhysConstPtr, PhysMutPtr};
use crate::virtmem::PageTable;

use allocators::Arena;
use core::panic::PanicInfo;
use fdt_rs::base::DevTree;
use ksync::SpinLock;
use libkernel::arch;
use libkernel::arch::cpu::{SScratch, SStatus, SStatusFlags, Satp};
use libkernel::arch::trap::{enable_interrupts, trap_frame_restore, TrapFrame};
use log::Level;
use sbi_log::KernelLogger;

pub struct InitCaps {
    mem: CSlot,
    init_task: CSlot,
}

impl InitCaps {
    const fn empty() -> Self {
        Self {
            mem: CSlot::empty(),
            init_task: CSlot::empty(),
        }
    }
}

static LOGGER: KernelLogger = KernelLogger::new(Level::Debug);

/// TODO: fix this somehow
/// CSlot isn't send because raw pointers... meh
unsafe impl Send for InitCaps {}

pub static INIT_CAPS: SpinLock<InitCaps> = SpinLock::new(InitCaps::empty());

pub static mut KERNEL_ROOT_PT: mem::PhysConstPtr<virtmem::PageTable> =
    mem::PhysConstPtr(0x0 as *const virtmem::PageTable);

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    // print panic message
    crate::println!("!!! Kernel Panic !!!\n  {}", info);

    // shutdown the device
    use sbi::system_reset::*;
    match system_reset(ResetType::Shutdown, ResetReason::SystemFailure) {
        Ok(_) => {}
        Err(e) => crate::println!("Shutdown error: {}", e),
    };
    arch::shutdown()
}

#[no_mangle]
extern "C" fn _start(
    _argc: u32,
    _argv: *const *const core::ffi::c_char,
    phys_fdt: PhysConstPtr<u8>,
    phys_mem_start: PhysMutPtr<u8>,
    phys_mem_end: PhysMutPtr<u8>,
) {
    LOGGER.install().expect("Could not install logger");
    log::info!("Hello world from the kernel!");
    let fdt_addr = mem::phys_to_kernel_ptr(phys_fdt);

    kernel_main(0, 0, fdt_addr, phys_mem_start, phys_mem_end);

    use sbi::system_reset::*;
    system_reset(ResetType::Shutdown, ResetReason::NoReason).unwrap();
    arch::shutdown();
}

extern "C" fn kernel_main(
    _hartid: usize,
    _unused: usize,
    dtb: *const u8,
    phys_mem_start: PhysMutPtr<u8>,
    phys_mem_end: PhysMutPtr<u8>,
) {
    // parse device tree from bootloader
    let _device_tree = unsafe { DevTree::from_raw_pointer(dtb).unwrap() };

    let kernel_root_pt = init_kernel_pagetable();
    unsafe {
        KERNEL_ROOT_PT = mem::kernel_to_phys_ptr(kernel_root_pt as *mut PageTable);
    }

    let mut allocator = init_alloc(phys_mem_start, phys_mem_end);

    let trap_stack = init_trap_handler_stack(&mut allocator);
    init_kernel_trap_handler(&mut allocator, trap_stack);

    log::debug!("enabled interrupts");
    enable_interrupts();

    log::debug!("creating init caps");
    init::create_init_caps(allocator);

    log::debug!("switching to userspace");
    run_init(trap_stack);
}

fn init_kernel_pagetable() -> &'static mut PageTable {
    // clean up userspace mapping from kernel loader
    log::debug!("Cleaning up userspace mapping from kernel loader");
    let root_pagetable_phys = (Satp::read().ppn << 12) as *mut PageTable;
    log::debug!("Kernel Pagetable Phys: {root_pagetable_phys:p}");
    let root_pt = unsafe {
        &mut *(mem::phys_to_kernel_usize(root_pagetable_phys as usize) as *mut PageTable)
    };
    virtmem::unmap_userspace(root_pt);
    unsafe {
        core::arch::asm!("sfence.vma");
    }
    return root_pt;
}

fn init_alloc(
    phys_mem_start: PhysMutPtr<u8>,
    phys_mem_end: PhysMutPtr<u8>,
) -> Arena<'static, Page> {
    log::debug!("start: {phys_mem_start:?}, end: {phys_mem_end:?}");
    let virt_start = phys_to_kernel_mut_ptr(phys_mem_start) as *mut Page;
    let virt_end = phys_to_kernel_mut_ptr(phys_mem_end) as *mut Page;
    log::debug!("virt_start: {virt_start:p} virt_end: {virt_end:p}");
    let mem_slice: &mut [Page] = unsafe {
        core::slice::from_raw_parts_mut(
            phys_to_kernel_mut_ptr(phys_mem_start) as *mut Page,
            (phys_to_kernel_mut_ptr(phys_mem_end) as usize
                - phys_to_kernel_mut_ptr(phys_mem_start) as usize)
                / mem::PAGESIZE,
        )
    };

    log::debug!("Init Kernel Allocator");
    let allocator = Arena::new(mem_slice);
    return allocator;
}

fn init_trap_handler_stack(allocator: &mut Arena<'static, Page>) -> *mut () {
    let trap_handler_stack: *mut Page = allocator.alloc_many_raw(10).unwrap().cast();
    let stack_start = unsafe { trap_handler_stack.add(10) as *mut () };
    log::debug!("trap_stack: {stack_start:p}");
    return stack_start;
}

fn init_kernel_trap_handler(allocator: &mut Arena<'static, Page>, trap_stack_start: *mut ()) {
    let trap_frame: *mut TrapFrame = allocator.alloc_one_raw().unwrap().cast();
    unsafe { (*trap_frame).trap_handler_stack = trap_stack_start as *mut usize };
    unsafe {
        SScratch::write(trap_frame as usize);
    }
    log::debug!("trap frame: {trap_frame:p}");
}

fn run_init(trap_stack: *mut ()) {
    unsafe {
        set_return_to_user();
        let mut guard = INIT_CAPS.try_lock().unwrap();
        let task = guard.init_task.cap.get_task_mut().unwrap();
        yield_to_task(trap_stack as *mut u8, task);
    };
}

/// Yield to the task that owns the given `trap_frame`
unsafe fn yield_to_task(trap_handler_stack: *mut u8, task: &mut caps::Cap<caps::Task>) -> ! {
    let state = unsafe { task.state.as_mut().unwrap() };
    let trap_frame = &mut state.frame;
    trap_frame.trap_handler_stack = trap_handler_stack.cast();
    let root_pt = state.vspace.cap.get_vspace_mut().unwrap().root;
    log::debug!("enabling task pagetable");
    unsafe {
        virtmem::use_pagetable(kernel_to_phys_mut_ptr(root_pt));
    }
    log::debug!("restoring trap frame");
    trap_frame_restore(trap_frame as *mut TrapFrame);
}

unsafe fn set_return_to_user() {
    log::debug!("clearing sstatus.SPP flag to enable returning to user code");
    SStatus::clear(SStatusFlags::SPP);
}
