//! An elf binary to set up virtual memory and load the kernel in high address ranges
//!
//! This is a program which serves the purpose of loading the actual kernel in the execution environment that
//! it expects.
//! This simplifies kernel development because the kernel can be programmed with certain more assumptions regarding
//! its memory layout and virtual addressing.
//! This assumption can of course only hold when a separate stage runs before the actual kernel which configures
//! virtual addressing and sets up all relevant memory areas.
//! That is done by this `kernel_loader` program.
//!
//! ## Boot Sequence
//!
//! When the kernel-loader is bootet, not much is known about the environment in which it is booted.
//! For an example layout see the figure below however neither the argument order nor the size of the spaces between them is guaranteed in any way.
//!
//! ```text
//!                                  Physical Memory Layout Example at Boot
//! ┌───┬────────────────────────┬───┬─────────────────┬───┬─────────────────┬───┬──────────────────┬───┐
//! │ … │ kernel_loader elf file │ … │ argc, argv data │ … │ kernel elf file │ … │ device tree data │ … │
//! └───┴────────────────────────┴───┴─────────────────┴───┴─────────────────┴───┴──────────────────┴───┘
//! ```
//!
//! The first thing to do is therefore to bring this layout into order by copying all data into space that was reserved inside the loaders data section.
//!
//! ```text
//!  Physical Memory Layout after Inlining
//!   ┌───┬────────────────────────┬───┐
//!   │ … │ kernel_loader elf file │ … │
//!   │   │ + argc, argv data      │   │
//!   │   │ + kernel elf file      │   │
//!   │   │ + device tree data     │   │
//!   └───┴────────────────────────┴───┘
//! ```
//!
//! Afterward, all argument data is evaluated and an allocator is set up which takes over further memory management.
//! All inlined data is now copied out of its inlined storage into properly allocated storage again.
//!
//! Now that memory management is set up using our own allocator, PageTables are created, virtual memory is turned on, the kernel elf binary is loaded and control is passed over to it.
//! All memory management information is passed to the kernel so that it can reuse it.
//!
#![no_std]
#![no_main]

mod args;
mod devtree;
mod elfloader;
mod trap;
mod user_args;
mod virtmem;

use crate::args::{CmdArgIter, LoaderArgs};
use crate::devtree::DeviceInfo;
use crate::elfloader::KernelLoader;
use crate::user_args::UserArgs;
use ::elfloader::ElfBinary;
use allocators::bump_allocator::{
    BackwardBumpingAllocator, BumpAllocator, ForwardBumpingAllocator,
};
use allocators::{AllocInit, Allocator, Box};
use core::alloc::Layout;
use core::arch::asm;
use core::cmp::min;
use core::mem::MaybeUninit;
use core::panic::PanicInfo;
use core::ptr;
use core::ptr::slice_from_raw_parts_mut;
use device_tree::fdt::FlattenedDeviceTree;
use klog::KernelLogger;
use log::Level;
use riscv::mem::mapping::PhysMapping;
use riscv::mem::{PageTable, PAGESIZE};

const DEFAULT_LOG_LEVEL: Level = Level::Debug;

static LOGGER: KernelLogger = KernelLogger::new(DEFAULT_LOG_LEVEL);

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    log::error!("!!! Kernel Loader Panic !!!\n  {}", info);
    riscv::power::abort();
}

/// The entry point of the loader that is called by U-Boot
#[no_mangle]
pub extern "C" fn _start(argc: u32, mut argv: *const *const core::ffi::c_char) -> ! {
    LOGGER.install().expect("Could not install logger");
    trap::set_trap_handler();

    // inline all arguments into temporary space reserved in this binary
    log::debug!("inlining all data to reorder memory");
    argv = unsafe { args::inline_args(argc, argv) };
    let args = LoaderArgs::from_args(CmdArgIter::from_argc_argv(argc, argv));
    let mut device_tree_ptr = unsafe { devtree::inline_devtree(args.phys_fdt_addr) };
    let kernel_elf_ptr = unsafe { elfloader::inline_elf_file(args.image_addr, args.image_size) };

    // parse device tree and extract relevant device information
    log::debug!("parsing device tree to get information about the host hardware");
    let device_info = unsafe {
        DeviceInfo::from_raw_ptr(device_tree_ptr).expect("Could not load device information")
    };
    log::debug!("got device info: {:x?}", device_info);

    // parse user specified arguments and apply them
    let user_args = match device_info.bootargs {
        None => UserArgs::default(),
        Some(args) => UserArgs::from_str(args),
    };
    LOGGER.update_log_level(user_args.log_level);
    log::debug!("got user arguments: {:?}", user_args);

    // extract usable memory from device information
    let (mem_start, mem_len) = device_info.usable_memory;

    // create an allocator to allocate essential data structures from the end of usable memory
    log::debug!(
        "creating allocator for general purpose memory start = {:p} end = {:0x} (len = {:0x} bytes)",
        mem_start,
        mem_start as usize + mem_len,
        mem_len,
    );
    let allocator = BackwardBumpingAllocator::<'static>::new(unsafe {
        core::slice::from_raw_parts_mut::<'_, u8>(mem_start, mem_len)
    });

    // copy argument data into memory allocated by the allocator
    argv = unsafe { args::copy_to_allocated_mem(&allocator) };
    device_tree_ptr = unsafe {
        devtree::copy_to_allocated_memory(&allocator, device_info.fdt.header.total_size as usize)
    };
    //assert!(unsafe { FlattenedDeviceTree::from_ptr(device_tree_ptr).is_ok());

    // allocate a root PageTable for the initial kernel execution environment
    log::debug!("creating kernels root PageTable");
    let root_pagetable = {
        let ptr = allocator
            .allocate(Layout::new::<PageTable>(), AllocInit::Uninitialized)
            .expect("Could not allocate memory for root page table")
            .as_mut_ptr()
            .cast::<MaybeUninit<PageTable>>();
        let ptr = PageTable::init(ptr);
        unsafe { ptr.as_mut().unwrap() }
    };

    // load the kernel ELF file
    log::debug!("loading kernel elf binary");
    let mut kernel_loader = KernelLoader::new(&allocator, root_pagetable, PhysMapping::identity());
    let binary =
        ElfBinary::new(args.get_kernel_bin()).expect("Could not load kernel as elf object");
    let entry_point = binary.entry_point();
    binary
        .load(&mut kernel_loader)
        .expect("Could not load the kernel elf binary into memory");

    const STACK_LOW: u64 = 0xfffffffffff70000;
    const STACK_SIZE: u64 = 0xf000;
    const STACK_HIGH: u64 = STACK_LOW + STACK_SIZE;
    kernel_loader.load_stack(STACK_LOW, STACK_HIGH);
    let KernelLoader {
        allocator,
        root_pagetable,
        ..
    } = kernel_loader;

    // a small hack, so that we don't run into problems when enabling virtual memory
    // TODO: the kernel has to clean up lower address space later
    virtmem::setup_lower_mem_id_map(root_pagetable, allocator);
    let _virt_phys_map = virtmem::setup_phys_mapping(root_pagetable, allocator);

    log::info!("enabling virtual memory!");
    unsafe {
        virtmem::use_pagetable(root_pagetable as *mut PageTable);
    }

    // TODO Don't move device-tree since it is located in a memory area that is outside of our allocation pool and fine to be there. However not moving it currently panics the kernel :(
    log::debug!("moving device tree (again -.-)");
    let mut phys_dev_tree = Box::new_uninit_slice_with_alignment(
        device_info.fdt.header.total_size as usize,
        4096,
        allocator,
    )
    .unwrap();
    let phys_dev_tree = unsafe {
        ptr::copy_nonoverlapping(
            device_info.fdt.buf.as_ptr(),
            phys_dev_tree.as_mut_ptr() as *mut u8,
            device_info.fdt.header.total_size as usize,
        );
        phys_dev_tree.assume_init()
    };
    assert!(FlattenedDeviceTree::from_buffer(&phys_dev_tree).is_ok());

    // waste a page or two so we get back to page alignment
    // TODO: remove this when the kernel fixes alignment itself
    let _x = allocator
        .allocate(
            Layout::from_size_align(PAGESIZE, PAGESIZE).unwrap(),
            AllocInit::Uninitialized,
        )
        .unwrap();

    // TODO: relocate and map argv
    // TODO: add phys mem to argv

    let phys_free_mem = allocator.steal_remaining_mem().as_mut_ptr_range();

    log::info!("starting Kernel, entry point: {entry_point:#x}");
    unsafe {
        asm!(
            "mv gp, x0",
            "mv sp, {stack}",
            "jr {entry}",
            stack = in(reg) STACK_HIGH - 16,
            entry = in(reg) entry_point,
            in("a0") argc,
            in("a1") argv,
            in("a2") phys_dev_tree.leak().as_mut_ptr(),
            in("a3") phys_free_mem.start,
            in("a4") phys_free_mem.end,
            options(noreturn)
        )
    }
}
