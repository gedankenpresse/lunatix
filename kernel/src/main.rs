#![no_std]
#![no_main]

#[path = "arch/riscv64imac/mod.rs"]
mod arch;

mod caps;
mod init;
mod mem;
mod printk;
mod userspace;

use crate::arch::trap::TrapFrame;
use crate::caps::CSlot;
use crate::mem::Page;
use crate::userspace::fake_userspace;
use core::panic::PanicInfo;
use fdt_rs::base::DevTree;
use ksync::SpinLock;
use memory::Arena;
use sifive_shutdown_driver::{ShutdownCode, SifiveShutdown};
use thiserror_no_std::private::DisplayAsDisplay;

struct InitCaps {
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

/// TODO: fix this somehow
/// CSlot isn't send because raw pointers... meh
unsafe impl Send for InitCaps {}

static INIT_CAPS: SpinLock<InitCaps> = SpinLock::new(InitCaps::empty());

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    // print panic message
    println!("!!! Kernel Panic !!!");
    println!("  {}", info.as_display());

    // shutdown the device
    unsafe {
        let shutdown_device = SifiveShutdown::from_ptr(0x100000 as *mut u32);
        shutdown_device.shutdown(ShutdownCode::Fail(1))
    }
}

fn get_memory(dev_tree: &DevTree) -> fdt_rs::error::Result<Option<(u64, u64)>> {
    use fdt_rs::base::DevTree;
    use fdt_rs::prelude::{FallibleIterator, PropReader};
    let mut nodes = dev_tree.nodes();
    let mut memory = None;
    while let Some(item) = nodes.next()? {
        if item.name()?.starts_with("memory") {
            memory = Some(item);
            break;
        }
    }
    let memory = match memory {
        Some(node) => node,
        None => panic!("no memory"),
    };

    println!("{:?}", memory.name()?);
    let mut props = memory.props();
    while let Some(prop) = props.next()? {
        if prop.name().unwrap() == "reg" {
            let start = prop.u64(0)?;
            let size = prop.u64(1)?;
            return Ok(Some((start, size)));
        }
    }
    return Ok(None);
}

fn init_heap(dev_tree: &DevTree) -> memory::Arena<'static, crate::mem::Page> {
    extern "C" {
        static mut _heap_start: u64;
    }

    let heap_start: *mut u8 = unsafe { &mut _heap_start as *mut u64 as *mut u8 };
    let (start, size) = get_memory(dev_tree).unwrap().unwrap();
    assert!(heap_start >= start as *mut u8);
    assert!(heap_start < (start + size) as *mut u8);
    let heap_size = size - (heap_start as u64 - start);
    let pages = heap_size as usize / crate::mem::PAGESIZE;
    assert!(pages * crate::mem::PAGESIZE <= (heap_size as usize));

    let pages =
        unsafe { core::slice::from_raw_parts_mut(heap_start as *mut crate::mem::Page, pages) };
    let mem = memory::Arena::new(pages);
    println!("{:?}", &mem);
    mem
}

/// Calculate the stack pointer from a given memory region that should be used as program stack
unsafe fn calc_stack_start(ptr: *mut Page, num_pages: usize) -> *mut u8 {
    ptr.add(num_pages).cast()
}

/// Yield to the task that owns the given `trap_frame`
unsafe fn yield_to(trap_handler_stack: *mut u8, trap_frame: &mut TrapFrame) -> ! {
    trap_frame.trap_stack = trap_handler_stack.cast();
    arch::trap::trap_frame_restore(trap_frame as *mut TrapFrame, trap_frame.ctx.epc);
}

unsafe fn set_return_to_user() {
    let spp: usize = 1 << 8;
    core::arch::asm!("csrc sstatus, a0",in("a0") spp);
}

// Fill INIT_CAPS with appropriate capabilities
fn create_init_caps(alloc: Arena<'static, Page>) {
    // create capability objects for userspace code
    let mut guard = INIT_CAPS.try_lock().unwrap();
    guard
        .mem
        .set(caps::Cap::from_content(caps::Memory { inner: alloc }))
        .unwrap();
    match &mut *guard {
        InitCaps { mem, init_task } => {
            caps::Task::init(init_task, mem.cap.get_memory_mut().unwrap())
        }
    }
    .unwrap();

    // setup stack for userspace code
    const NUM_PAGES: usize = 1;
    let stack = guard
        .mem
        .cap
        .get_memory_mut()
        .unwrap()
        .alloc_pages_raw(NUM_PAGES)
        .unwrap();
    let task = guard.init_task.cap.get_task_mut().unwrap();
    let task_state = unsafe { &mut *task.state };
    task_state.frame.general_purpose_regs.registers[2] =
        unsafe { calc_stack_start(stack, NUM_PAGES) as usize };

    // set up program counter to point to userspace code
    let userspace_pc = fake_userspace as *const u8 as usize;
    task_state.frame.ctx.epc = userspace_pc;
}

#[no_mangle]
extern "C" fn kernel_main(_hartid: usize, _unused: usize, dtb: *mut u8) {
    // parse device tree from bootloader
    let device_tree = unsafe { DevTree::from_raw_pointer(dtb).unwrap() };

    // setup page heap
    // after this operation, the device tree was overwritten
    let mut allocator = init_heap(&device_tree);
    drop(device_tree);
    drop(dtb);

    println!("{:?}", arch::cpu::Sie::read());

    // setup context switching
    let trap_handler_stack: *mut Page = allocator.alloc_many_raw(10).unwrap().cast();
    arch::trap::enable_interrupts();

    println!("{:?}", arch::cpu::Sie::read());

    create_init_caps(allocator);
    // switch to userspace
    unsafe {
        set_return_to_user();
        let mut guard = INIT_CAPS.try_lock().unwrap();
        let task = guard.init_task.cap.get_task_mut().unwrap();
        let taskstate = task.state;
        drop(guard);
        let frame = &mut (*taskstate).frame;
        yield_to(trap_handler_stack as *mut u8, frame)
    };

    // shut down the machine
    let shutdown_device: &mut SifiveShutdown = unsafe { &mut *(0x100_000 as *mut SifiveShutdown) };
    unsafe { shutdown_device.shutdown(ShutdownCode::Pass) };
}
