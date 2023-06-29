#![no_std]
#![no_main]

#[path = "arch/riscv64imac/mod.rs"]
mod arch;

mod caps;
mod init;
mod logging;
mod mem;
mod virtmem;

use crate::arch::cpu::SStatusFlags;
use crate::arch::trap::TrapFrame;
use crate::caps::CSlot;
use crate::logging::KernelLogger;
use crate::mem::Page;
use core::panic::PanicInfo;
use fdt_rs::base::DevTree;
use ksync::SpinLock;
use log::Level;
use sifive_shutdown_driver::{ShutdownCode, SifiveShutdown};

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
    log::error!("!!! Kernel Panic !!!\n  {}", info);

    // shutdown the device
    unsafe {
        let shutdown_device = SifiveShutdown::from_ptr(0x100000 as *mut u32);
        shutdown_device.shutdown(ShutdownCode::Fail(1))
    }
}

fn get_memory(dev_tree: &DevTree) -> fdt_rs::error::Result<Option<(u64, u64)>> {
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

    log::debug!("[init alloc] {heap_start:p} {heap_size:0x}");
    let pages = heap_size as usize / crate::mem::PAGESIZE;
    assert!(pages * crate::mem::PAGESIZE <= (heap_size as usize));

    let pages =
        unsafe { core::slice::from_raw_parts_mut(heap_start as *mut crate::mem::Page, pages) };
    let mem = memory::Arena::new(pages);
    log::debug!("{:?}", &mem);
    mem
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

struct CmdArgIter {
    argc: u32,
    current: u32,
    argv: *const *const core::ffi::c_char,
}

impl Iterator for CmdArgIter {
    type Item = &'static str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.argc {
            return None;
        }
        let i = self.current;
        self.current += 1;
        let cstr = unsafe { *self.argv.offset(i as isize) };
        use core::ffi::CStr;
        let cs = unsafe { CStr::from_ptr(cstr) };
        let s = cs.to_str().unwrap();
        return Some(s);
    }
}

fn arg_iter(
    argc: u32,
    argv: *const *const core::ffi::c_char,
) -> impl Iterator<Item = &'static str> {
    CmdArgIter {
        argc,
        argv,
        current: 0,
    }
}

#[no_mangle]
extern "C" fn kernel_main_elf(argc: u32, argv: *const *const core::ffi::c_char) {
    log::info!("Hello world from the kernel!");

    let mut fdt_addr: Option<*const u8> = None;
    for arg in arg_iter(argc, argv) {
        if let Some(addr_str) = arg.strip_prefix("fdt_addr=") {
            let addr = u64::from_str_radix(addr_str, 16).unwrap();
            fdt_addr = Some(addr as *const u8);
            log::debug!("got fdt addr: {:p}", fdt_addr.unwrap());
        } else {
            log::debug!("unknown arg {}", arg);
        }
    }

    kernel_main(0, 0, fdt_addr.expect("need fdt_addr arg to start"));
    // shut down the machine
    arch::shutdown();
    let shutdown_device: &mut SifiveShutdown = unsafe { &mut *(0x100_000 as *mut SifiveShutdown) };
    unsafe { shutdown_device.shutdown(ShutdownCode::Pass) };
}

#[no_mangle]
#[allow(unreachable_code)]
extern "C" fn kernel_main(_hartid: usize, _unused: usize, dtb: *const u8) {
    LOGGER.install().expect("Could not install logger");
    // parse device tree from bootloader
    let device_tree = unsafe { DevTree::from_raw_pointer(dtb).unwrap() };

    // save memory for later
    // we need this to map all physical memory into the kernelspace when enabling virtual memory
    log::debug!("physical memory:\t");
    let (mem_start, mem_length) = get_memory(&device_tree).unwrap().unwrap();
    log::debug!("{mem_start:0x} {mem_length:0x}");

    // setup page heap
    // after this operation, the device tree was overwritten
    log::debug!("init alloc");
    let mut allocator = init_heap(&device_tree);
    drop(device_tree);
    drop(dtb);

    // setup context switching
    let trap_handler_stack: *mut Page = allocator.alloc_many_raw(10).unwrap().cast();
    let trap_frame: *mut TrapFrame = allocator.alloc_one_raw().unwrap().cast();
    unsafe { (*trap_frame).trap_handler_stack = trap_handler_stack.add(10).cast() }

    log::debug!("enabling interrupts");
    unsafe {
        arch::cpu::SScratch::write(trap_frame as usize);
    }
    arch::trap::enable_interrupts();

    // println!("[main] testing interrupts");
    // unsafe { *(0x4 as *mut u8) = 0 };

    log::debug!("creating kernel id map");
    let kernel_root =
        virtmem::create_kernel_page_table(&mut allocator, mem_start as usize, mem_length as usize)
            .expect("Could not create kernel page table");
    log::debug!("enabling virtual memory");
    unsafe { KERNEL_ROOT_PT = kernel_root as *const virtmem::PageTable };
    unsafe {
        virtmem::use_pagetable(kernel_root);
    }

    log::debug!("creating init caps");
    init::create_init_caps(allocator);

    // switch to userspace
    log::debug!("switching to userspace");
    unsafe {
        set_return_to_user();
        let mut guard = INIT_CAPS.try_lock().unwrap();
        let task = guard.init_task.cap.get_task_mut().unwrap();
        yield_to_task(trap_handler_stack as *mut u8, task);
    };

    // shut down the machine
    let shutdown_device: &mut SifiveShutdown = unsafe { &mut *(0x100_000 as *mut SifiveShutdown) };
    unsafe { shutdown_device.shutdown(ShutdownCode::Pass) };
}
