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
mod user_args;
mod virtmem;

use crate::args::{CmdArgIter, LoaderArgs};
use crate::devtree::DeviceInfo;
use crate::elfloader::KernelLoader;
use crate::user_args::UserArgs;
use crate::virtmem::virt_to_phys;
use ::elfloader::ElfBinary;
use allocators::bump_allocator::{BumpAllocator, ForwardBumpingAllocator};
use allocators::{AllocInit, Allocator, Box};
use bitflags::Flags;
use core::alloc::Layout;
use core::cmp::min;
use core::panic::PanicInfo;
use core::ptr;
use device_tree::fdt::FlattenedDeviceTree;
use klog::{println, KernelLogger};
use log::Level;
use riscv::mem::{
    paddr_from_parts, paddr_ppn_segments, vaddr_page_offset, vaddr_vpn_segments, EntryFlags, PAddr,
    VAddr,
};
use riscv::pt::{PageTable, PAGESIZE};

const DEFAULT_LOG_LEVEL: Level = Level::Info;

static LOGGER: KernelLogger = KernelLogger::new(DEFAULT_LOG_LEVEL);

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    log::error!("!!! Kernel Loader Panic !!!\n  {}", info);
    riscv::power::abort();
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
    log::debug!("device info = {:x?}", device_info);

    // parse user specified arguments and apply them
    let user_args = match device_info.bootargs {
        None => UserArgs::default(),
        Some(args) => UserArgs::from_str(args),
    };
    LOGGER.update_log_level(user_args.log_level);
    log::debug!("got user arguments {:?}", user_args);

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

    log::info!("validating root pagetable {:?}", root_pagetable);
    //validate_pagetable(root_pagetable, 0);

    //let entry = &mut root_pagetable.entries[508];
    //unsafe { entry.set(riscv::mem::paddr_ppn(u64::MAX), EntryFlags::empty()) };
    //log::info!("complete entry: 0b{:064b}", entry);
    //log::info!("addr only:      0b{:064b}", entry.get_addr().unwrap());

    let pc = riscv::cpu::PC::read();
    let pc_phys = translate_addr(&root_pagetable, pc);
    log::info!("pc = {pc:x}    translated_pc = {pc_phys:x}");

    panic!();

    log::info!("enabling virtual memory!");
    unsafe {
        virtmem::use_pagetable(root_pagetable as *mut PageTable);
    }

    panic!("exit");

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

fn validate_pagetable(pt: &PageTable, current_level: usize) {
    for (i, entry) in pt.entries.iter().enumerate() {
        if !entry.is_valid() {
            continue;
        }

        if entry.is_leaf() {
            log::debug!("found leaf entry {i:3} at level {current_level}: {entry:?}");
        } else {
            log::debug!(
                "found pointer from level {current_level} to level {}: {i:3} {entry:?}",
                current_level + 1
            );
            let next_pt_addr = entry.get_addr().unwrap();
            let next_pt = unsafe { (next_pt_addr as *const PageTable).as_ref().unwrap() };
            validate_pagetable(next_pt, current_level + 1);
        }
    }
}

fn translate_addr(pt: &PageTable, vaddr: VAddr) -> PAddr {
    let page_offset = vaddr_page_offset(vaddr);
    let [vpn0, vpn1, vpn2] = vaddr_vpn_segments(vaddr);
    log::info!("vaddr = {vaddr:x}    vpn0 = {vpn0:x}    vpn1 = {vpn1:x}    vpn2 = {vpn2:x}    offset = {page_offset:x}");

    let pte = &pt.entries[vpn0 as usize];
    log::info!("level 0 = {pte:?}");
    if pte.is_leaf() {
        let entry_ppns = paddr_ppn_segments(pte.get_addr().unwrap());
        return paddr_from_parts([entry_ppns[0], vpn1, vpn2], page_offset);
    }
    let pt = unsafe {
        (pte.get_addr().unwrap() as *const PageTable)
            .as_ref()
            .unwrap()
    };

    let pte = &pt.entries[vpn0 as usize];
    log::info!("level 1 = {pte:?}");
    let pt = unsafe {
        (pte.get_addr().unwrap() as *const PageTable)
            .as_ref()
            .unwrap()
    };

    let pte = &pt.entries[vpn0 as usize];
    log::info!("level 2 = {pte:?}");
    let ppn = pte.get_addr().unwrap();

    ppn | page_offset
}
