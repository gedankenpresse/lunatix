#![no_std]
#![no_main]
// TODO: remove dead code
#![allow(dead_code)]
#![allow(unused_variables)]

mod caps;
mod init;
mod ipc;
mod sched;
mod trap;
mod uapi;
mod virtmem;

use crate::caps::CSlot;

use allocators::Arena;
use core::panic::PanicInfo;
use core::slice;
use fdt_rs::base::DevTree;
use ksync::SpinLock;
use libkernel::arch::cpu;
use libkernel::arch::cpu::{InterruptBits, SScratch, SStatus, SStatusFlags, Satp, SatpMode};
use libkernel::arch::trap::{enable_interrupts, trap_frame_restore, TrapFrame};
use libkernel::mem::ptrs::{MappedConstPtr, MappedMutPtr, PhysConstPtr, PhysMutPtr};
use libkernel::mem::{MemoryPage, PageTable, PAGESIZE, VIRT_MEM_KERNEL_START};
use libkernel::sbi_log::KernelLogger;
use libkernel::{arch, println};
use log::Level;

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

static LOGGER: KernelLogger = KernelLogger::new(Level::Trace);

/// TODO: fix this somehow
/// CSlot isn't send because raw pointers... meh
unsafe impl Send for InitCaps {}

pub static INIT_CAPS: SpinLock<InitCaps> = SpinLock::new(InitCaps::empty());

pub static mut KERNEL_ROOT_PT: PhysConstPtr<PageTable> = PhysConstPtr::null();

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    // print panic message
    println!("!!! Kernel Panic !!!\n  {}", info);

    // shutdown the device
    use sbi::system_reset::*;
    match system_reset(ResetType::Shutdown, ResetReason::SystemFailure) {
        Ok(_) => {}
        Err(e) => println!("Shutdown error: {}", e),
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
    assert_start_expectations();

    let fdt_addr = phys_fdt.as_mapped();

    kernel_main(0, 0, fdt_addr.into(), phys_mem_start, phys_mem_end);

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
    unsafe { KERNEL_ROOT_PT = MappedConstPtr::from(kernel_root_pt as *const PageTable).as_direct() }

    let mut allocator = init_alloc(phys_mem_start, phys_mem_end);

    let trap_stack = init_trap_handler_stack(&mut allocator);
    init_kernel_trap_handler(&mut allocator, trap_stack);

    log::debug!("creating init caps");
    init::create_init_caps(allocator);

    log::debug!("enabling interrupts");
    //arch::timers::set_next_timer(0).unwrap();
    enable_interrupts();

    log::debug!("switching to userspace");
    run_init(trap_stack);
}

fn init_kernel_pagetable() -> &'static mut PageTable {
    // clean up userspace mapping from kernel loader
    log::debug!("Cleaning up userspace mapping from kernel loader");
    let root_pagetable_phys = (Satp::read().ppn << 12) as *mut PageTable;
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

fn init_alloc(
    phys_mem_start: PhysMutPtr<u8>,
    phys_mem_end: PhysMutPtr<u8>,
) -> Arena<'static, MemoryPage> {
    log::debug!("start: {phys_mem_start:?}, end: {phys_mem_end:?}");
    let virt_start = phys_mem_start.as_mapped().raw();
    let virt_end = phys_mem_end.as_mapped().raw();
    log::debug!("virt_start: {virt_start:p} virt_end: {virt_end:p}");
    let mem_slice: &mut [MemoryPage] = unsafe {
        slice::from_raw_parts_mut(
            virt_start.cast::<MemoryPage>(),
            (virt_end as usize - virt_start as usize) / PAGESIZE,
        )
    };

    log::debug!("Init Kernel Allocator");
    let allocator = Arena::new(mem_slice);
    return allocator;
}

fn init_trap_handler_stack(allocator: &mut Arena<'static, MemoryPage>) -> *mut () {
    let trap_handler_stack: *mut MemoryPage = allocator.alloc_many_raw(10).unwrap().cast();
    let stack_start = unsafe { trap_handler_stack.add(10) as *mut () };
    log::debug!("trap_stack: {stack_start:p}");
    return stack_start;
}

fn init_kernel_trap_handler(allocator: &mut Arena<'static, MemoryPage>, trap_stack_start: *mut ()) {
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
        let mut task = &mut guard.init_task;
        yield_to_task(trap_stack as *mut u8, &mut task);
    };
}

/// Yield to the task that owns the given `trap_frame`
unsafe fn yield_to_task(trap_handler_stack: *mut u8, task: &mut caps::CSlot) -> ! {
    let taskref = task.get_task_mut().unwrap();
    unsafe {
        crate::sched::set_active_task(taskref.state);
    }
    let state = unsafe { taskref.state.as_mut().unwrap() };
    let trap_frame = &mut state.frame;
    trap_frame.trap_handler_stack = trap_handler_stack.cast();
    let root_pt = state.vspace.get_vspace_mut().unwrap().root;
    log::debug!("enabling task pagetable");
    unsafe {
        virtmem::use_pagetable(MappedMutPtr::from(root_pt).as_direct());
    }
    log::debug!("restoring trap frame");
    trap_frame_restore(trap_frame as *mut TrapFrame);
}

unsafe fn set_return_to_user() {
    log::debug!("clearing sstatus.SPP flag to enable returning to user code");
    SStatus::clear(SStatusFlags::SPP);
}

/// Assert that all environment conditions under which the kernel expects to be started are met
fn assert_start_expectations() {
    // check address translation
    assert_eq!(
        Satp::read().mode,
        SatpMode::Sv39,
        "kernel was booted with unsupported address translation mode {:?}",
        Satp::read().mode
    );

    // check that the kernel code was loaded into high memory
    assert!(
        kernel_main as *const u8 as usize >= VIRT_MEM_KERNEL_START,
        "kernel code was not loaded into high memory"
    );
    let dummy = 0u8;
    assert!(
        &dummy as *const u8 as usize >= VIRT_MEM_KERNEL_START,
        "kernel stack is not located in high memory"
    );

    // check that interrupts are not yet enabled
    assert_eq!(
        cpu::Sie::read(),
        InterruptBits::empty(),
        "kernel was started with interrupts already enabled"
    );
}
