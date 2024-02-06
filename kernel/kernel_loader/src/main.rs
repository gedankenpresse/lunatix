//! An elf binary to setup virtual memory and load the kernel in high address ranges
//!
//! This is a program which serves the simple purpose of loading the actual kernel in the execution environment that
//! it expects.
//! This simplifies kernel development because the kernel can be programmed and compiled with the assumption that
//! virtual addressing is already turned and never turned off.
//! This assumption can of course only hold when a separate stage runs before the actual kernel which configures
//! virtual addressing and loads the kernel binary at the addresses which it expects.
//! That is done by this `kernel_loader` program.
#![no_std]
#![no_main]

mod args;
mod devtree;
mod elfloader;
mod virtmem;

use crate::args::{CmdArgIter, LoaderArgs};
use crate::devtree::DeviceInfo;
use crate::elfloader::KernelLoader;
use ::elfloader::ElfBinary;
use allocators::bump_allocator::{
    BackwardBumpingAllocator, BumpAllocator, ForwardBumpingAllocator,
};
use allocators::{AllocInit, Allocator, Box};
use core::alloc::Layout;
use core::cmp::min;
use core::panic::PanicInfo;
use core::ptr;
use device_tree::fdt::FlattenedDeviceTree;
use klog::KernelLogger;
use log::Level;
use riscv::pt::{PageTable, PAGESIZE};
use sbi::system_reset::{ResetReason, ResetType};

static LOGGER: KernelLogger = KernelLogger::new(Level::Debug);

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    // print panic message
    log::error!("!!! Kernel Loader Panic !!!\n  {}", info);

    // try to shutdown (but loop in case that fails)
    let _ = sbi::system_reset::system_reset(ResetType::Shutdown, ResetReason::SystemFailure);
    log::error!("Could not shutdown device, looping now…");
    loop {}
}

/// The entry point of the loader that is called by U-Boot
#[no_mangle]
pub extern "C" fn _start(argc: u32, argv: *const *const core::ffi::c_char) -> ! {
    LOGGER.install().expect("Could not install logger");
    let args = LoaderArgs::from_args(CmdArgIter::from_argc_argv(argc, argv));
    log::debug!("kernel parameters = {:x?}", args);

    log::debug!("parsing device tree to get information about the host hardware");
    let device_info = unsafe {
        DeviceInfo::from_raw_ptr(args.phys_fdt_addr).expect("Could not load device information")
    };

    // extract usable memory from device information
    let (mem_start, mem_len) = device_info.usable_memory;
    let mut mem_end = unsafe { mem_start.add(mem_len) };

    // u-boot places the device tree and kernel arguments at the very end of physical memory and since we don't want
    // to overwrite it, we fake mem_end to end before it
    mem_end = (min(mem_end, args.phys_fdt_addr.cast_mut()) as usize & !(4096 - 1)) as *mut u8;

    // create an allocator to allocate essential data structures from the end of usable memory
    log::debug!(
        "creating allocator for general purpose memory start = {:p} end = {:p} (len = {} bytes)",
        mem_start,
        mem_end,
        mem_end as usize - mem_start as usize
    );
    let allocator = unsafe { ForwardBumpingAllocator::<'static>::new_raw(mem_start, mem_end) };

    // allocate a root PageTable for the initial kernel execution environment
    log::trace!("allocating kernels root PageTable");
    let root_table_box: Box<'_, '_, PageTable> = unsafe {
        Box::new_zeroed(&allocator)
            .expect("Could not setup root PageTable")
            .assume_init()
    };
    let root_table = root_table_box.leak();
    log::trace!("root_table addr: {:p}", root_table);

    // load the kernel ELF file
    let mut kernel_loader = KernelLoader::new(&allocator, root_table);
    let binary =
        ElfBinary::new(args.get_kernel_bin()).expect("Could not load kernel as elf object");
    binary
        .load(&mut kernel_loader)
        .expect("Could not load the kernel elf binary into memory");

    const STACK_LOW: usize = 0xfffffffffff70000;
    const STACK_SIZE: usize = 0xf000;
    const STACK_HIGH: usize = STACK_LOW + STACK_SIZE;
    kernel_loader.load_stack(STACK_LOW, STACK_HIGH);
    let entry_point = binary.entry_point();
    let KernelLoader {
        allocator,
        root_pagetable,
    } = kernel_loader;

    // a small hack, so that we don't run into problems when enabling virtual memory
    // TODO: the kernel has to clean up lower address space later
    log::debug!("identity mapping lower memory region");
    virtmem::id_map_lower_huge(root_pagetable);

    log::debug!("mapping physical memory to kernel");
    virtmem::kernel_map_phys_huge(root_pagetable);

    log::info!("enabling virtual memory!");
    unsafe {
        virtmem::use_pagetable(root_pagetable as *mut PageTable);
    }

    // TODO Don't move device-tree since it is located in a memory area that is outside of our allocation pool and fine to be there. However not moving it currently panics the kernel :(
    log::debug!("moving device tree");
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
    log::debug!("{:?}", phys_free_mem);

    log::info!("starting Kernel, entry point: {entry_point:0x}");
    unsafe {
        core::arch::asm!(
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
        );
    }

    unreachable!()
}
