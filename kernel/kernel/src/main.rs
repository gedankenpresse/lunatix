#![no_std]
#![no_main]

#[path = "arch/riscv64imac/mod.rs"]
mod arch;

mod caps;
mod init;
mod logging;
mod printk;
mod mem;
mod virtmem;




use crate::arch::cpu::Satp;
use crate::virtmem::{PageTable, virt_to_phys};
use crate::{arch::cpu::SStatusFlags, mem::phys_to_kernel};
use crate::arch::trap::TrapFrame;
use crate::caps::CSlot;
use crate::logging::KernelLogger;
use crate::mem::{Page, phys_to_kernel_mut_ptr};
use core::panic::PanicInfo;
use fdt_rs::base::DevTree;
use ksync::SpinLock;
use log::Level;
use mem::{PhysConstPtr, PhysMutPtr};
use memory::Arena;

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

pub static mut KERNEL_ROOT_PT: *const virtmem::PageTable = 0x0 as *const virtmem::PageTable;

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    // print panic message
    crate::println!("!!! Kernel Panic !!!\n  {}", info);

    // shutdown the device
    use sbi::system_reset::*;
    match system_reset(ResetType::Shutdown, ResetReason::SystemFailure) {
        Ok(_) => {},
        Err(e) => crate::println!("Shutdown error: {}", e),
    };
    arch::shutdown()
}

/// Yield to the task that owns the given `trap_frame`
unsafe fn yield_to_task(trap_handler_stack: *mut u8, task: &mut caps::Cap<caps::Task>) -> ! {
    let state = unsafe { task.state.as_mut().unwrap() };
    let trap_frame = &mut state.frame;
    trap_frame.trap_handler_stack = trap_handler_stack.cast();
    let root_pt = state.vspace.cap.get_vspace_mut().unwrap().root;
    log::debug!("enabling task pagetable");
    unsafe {
        virtmem::use_pagetable(root_pt);
    }
    log::debug!("restoring trap frame");
    arch::trap::trap_frame_restore(trap_frame as *mut TrapFrame);
}

unsafe fn set_return_to_user() {
    log::debug!("clearing sstatus.SPP flag to enable returning to user code");
    arch::cpu::SStatus::clear(SStatusFlags::SPP);
}


#[no_mangle]
extern "C" fn kernel_main_elf(
    argc: u32,
    argv: *const *const core::ffi::c_char,
    phys_fdt: PhysConstPtr<u8>,
    phys_mem_start: PhysMutPtr<u8>,
    phys_mem_end: PhysMutPtr<u8>,
) {
    LOGGER.install().expect("Could not install logger");
    log::info!("Hello world from the kernel!");
    let fdt_addr = mem::phys_to_kernel_ptr(phys_fdt);


    kernel_main(
        0,
        0,
        fdt_addr,
        phys_mem_start,
        phys_mem_end,
    );
    // shut down the machine

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
    let device_tree = unsafe { DevTree::from_raw_pointer(dtb).unwrap() };

    // setup page heap
    // after this operation, the device tree was overwritten
    let virt_start = phys_to_kernel_mut_ptr(phys_mem_start) as *mut Page;
    let virt_end = phys_to_kernel_mut_ptr(phys_mem_end) as *mut Page;
    log::debug!("virt_start: {virt_start:p} virt_end: {virt_end:p}");
    let mem_slice: &mut [Page] = unsafe { 
        core::slice::from_raw_parts_mut(
            phys_to_kernel_mut_ptr(phys_mem_start) as *mut Page,
            (phys_to_kernel_mut_ptr(phys_mem_end) as usize - phys_to_kernel_mut_ptr(phys_mem_start) as usize) / mem::PAGESIZE,
        )
    };

    // clean up userspace mapping from kernel loader
    log::debug!("Cleaning up userspace mapping from kernel loader");
    let root_pagetable_phys = (Satp::read().ppn << 12) as *mut PageTable;
    unsafe { KERNEL_ROOT_PT = root_pagetable_phys; }
    let root_pt = unsafe { &mut *(mem::phys_to_kernel_usize(root_pagetable_phys as usize) as *mut PageTable)  };
    virtmem::unmap_userspace(root_pt);
    unsafe { core::arch::asm!("sfence.vma"); }

    log::debug!("Init Kernel Allocator");
    let mut allocator = Arena::new(mem_slice);
    // setup context switching
    let trap_handler_stack: *mut Page = allocator.alloc_many_raw(10).unwrap().cast();
    let trap_frame: *mut TrapFrame = allocator.alloc_one_raw().unwrap().cast();
    unsafe { (*trap_frame).trap_handler_stack = trap_handler_stack.add(10) as *mut usize }
    unsafe { arch::cpu::SScratch::write(trap_frame as usize); }
    log::debug!("trap frame: {trap_frame:p} trap_stack: {trap_handler_stack:p}");
    arch::trap::enable_interrupts();
    log::debug!("enabled interrupts");


    unsafe { *(0x1 as *mut u8) = 0};
    // TODO: remove userspace from kernel page table

    log::debug!("creating init caps");
    init::create_init_caps(allocator);
    log::debug!("switching to userspace");
    // switch to userspace
    unsafe {
        set_return_to_user();
        let mut guard = INIT_CAPS.try_lock().unwrap();
        let task = guard.init_task.cap.get_task_mut().unwrap();
        yield_to_task(trap_handler_stack as *mut u8, task);
    };
}
