//! An elf binary to setup virtual memory and load the kernel in high address ranges
#![no_std]
#![no_main]

mod elfloader;
mod virtmem;

use crate::elfloader::KernelLoader;
use crate::virtmem::PageTable;
use ::elfloader::ElfBinary;
use allocators::{AllocInit, BumpAllocator, BumpBox};
use core::panic::PanicInfo;
use core::ptr::eq;
use fdt_rs::base::DevTree;
use libkernel::mem::PAGESIZE;
use log::Level;
use sbi_log::KernelLogger;

static LOGGER: KernelLogger = KernelLogger::new(Level::Debug);

static KERNEL_BIN: &[u8] =
    include_bytes!("../../../target/riscv64imac-unknown-none-elf/debug/kernel");

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    // print panic message
    log::error!("!!! Kernel Loader Panic !!!\n  {}", info);
    loop {}
}

struct Args {
    phys_fdt_addr: *const u8,
}

impl Args {
    fn from_args(args: impl Iterator<Item = &'static str>) -> Self {
        let mut phys_fdt_addr = None;
        for arg in args {
            if let Some(addr_s) = arg.strip_prefix("fdt_addr=") {
                let addr =
                    usize::from_str_radix(addr_s, 16).expect("fdt_addr should be in base 16");
                phys_fdt_addr = Some(addr as *const u8);
            }
        }
        Self {
            phys_fdt_addr: phys_fdt_addr.expect("no fdt_addr given"),
        }
    }
}

/// The entry point of the loader that is called by U-Boot
#[no_mangle]
pub extern "C" fn _start(argc: u32, argv: *const *const core::ffi::c_char) -> ! {
    LOGGER.install().expect("Could not install logger");
    let args = Args::from_args(libkernel::argv_iter::arg_iter(argc, argv));

    const GB: usize = 1024 * 1024 * 1024;
    const MEM_START: usize = 0x8000_0000 + GB / 2; // we just chose a high value that is larger than the kernel_loader binary
    const MEM_END: usize = 0xc0000000; // if we give 1GB of memory during qemu start, this is the last address
    let mut allocator = unsafe { BumpAllocator::new_raw(MEM_START as *mut u8, MEM_END as *mut u8) };

    let root_table = unsafe {
        PageTable::empty(&mut allocator)
            .expect("Could not setup root pagetable")
            .as_mut()
            .unwrap()
    };
    log::debug!("root_table addr: {:p}", root_table);

    let mut kernel_loader = KernelLoader::new(allocator, root_table);
    let binary = ElfBinary::new(KERNEL_BIN).expect("Could not load kernel as elf object");
    binary
        .load(&mut kernel_loader)
        .expect("Could not load the kernel elf binary into memory");
    let stack_start: usize = 0xfffffffffff7a000;
    kernel_loader.load_stack(stack_start - 0x5000, stack_start);
    let entry_point = binary.entry_point();
    let KernelLoader {
        mut allocator,
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

    log::debug!("parsing device tree");
    let device_tree = unsafe { DevTree::from_raw_pointer(args.phys_fdt_addr).unwrap() };

    log::debug!("moving device tree");
    let mut phys_dev_tree = BumpBox::new_uninit_slice(device_tree.buf().len(), &allocator).unwrap();
    let phys_dev_tree = unsafe {
        for (i, &byte) in device_tree.buf().iter().enumerate() {
            phys_dev_tree[i].write(byte);
        }
        phys_dev_tree.assume_init()
    };

    assert!(unsafe { DevTree::new(&phys_dev_tree) }.is_ok());

    // waste a page or two so we get back to page alignment
    let x = allocator
        .allocate(PAGESIZE, PAGESIZE, AllocInit::Uninitialized)
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
            stack = in(reg) stack_start - 16,
            entry = in(reg) entry_point,
            in("a0") argc,
            in("a1") argv,
            in("a2") phys_dev_tree.into_raw() as *mut u8,
            in("a3") phys_free_mem.start,
            in("a4") phys_free_mem.end,
        );
    }

    unreachable!()
}
