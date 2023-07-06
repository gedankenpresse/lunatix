//! An elf binary to setup virtual memory and load the kernel in high address ranges
#![no_std]
#![no_main]

mod elfloader;
mod virtmem;

use crate::elfloader::KernelLoader;
use crate::virtmem::PageTable;
use ::elfloader::ElfBinary;
use allocators::bump_allocator::{BackwardBumpingAllocator, BumpAllocator, BumpBox};
use allocators::AllocInit;
use core::cmp::min;
use core::mem::MaybeUninit;
use core::ops::Add;
use core::panic::PanicInfo;
use fdt_rs::base::DevTree;
use libkernel::device_info::DeviceInfo;
use libkernel::mem::PAGESIZE;
use libkernel::sbi_log::KernelLogger;
use log::Level;
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

struct Args {
    phys_fdt_addr: *const u8,
    image_addr: *const u8,
    image_size: Option<usize>,
}

impl Args {
    fn from_args(args: impl Iterator<Item = &'static str>) -> Self {
        let mut phys_fdt_addr = None;
        let mut image_addr = None;
        let mut image_size = None;
        for arg in args {
            if let Some(addr_s) = arg.strip_prefix("fdt_addr=") {
                let addr =
                    usize::from_str_radix(addr_s, 16).expect("fdt_addr should be in base 16");
                phys_fdt_addr = Some(addr as *const u8);
            }
            if let Some(addr_s) = arg.strip_prefix("image_addr=") {
                let addr =
                    usize::from_str_radix(addr_s, 16).expect("image_addr should be in base 16");
                image_addr = Some(addr as *const u8);
            }
            if let Some(size_s) = arg.strip_prefix("image_size=") {
                let size =
                    usize::from_str_radix(size_s, 16).expect("image size should be in base 16");
                image_size = Some(size);
            }
        }
        Self {
            phys_fdt_addr: phys_fdt_addr.expect("no fdt_addr given"),
            image_addr: image_addr.expect("no kernel image addr given"),
            image_size,
        }
    }

    fn get_kernel_bin(&self) -> &[u8] {
        const MB: usize = 1024 * 1024;
        unsafe { core::slice::from_raw_parts(self.image_addr, self.image_size.unwrap_or(2 * MB)) }
    }
}

/// The entry point of the loader that is called by U-Boot
#[no_mangle]
pub extern "C" fn _start(argc: u32, argv: *const *const core::ffi::c_char) -> ! {
    LOGGER.install().expect("Could not install logger");
    let args = Args::from_args(libkernel::argv_iter::arg_iter(argc, argv));

    log::debug!("parsing device tree to get information about the host hardware");
    let mut device_info_index = [0u8; 10 * 1024]; // TODO The size was tested to work in our qemu boot environment but is not properly chosen
    let device_info =
        unsafe { DeviceInfo::from_device_tree(args.phys_fdt_addr, &mut device_info_index) }
            .expect("Could not parse device info from flattened device tree");

    // extract usable memory from device information
    let (mem_start, mem_len) = device_info
        .get_usable_memory()
        .expect("Could not get usable memory from device tree");
    let mem_end = unsafe { mem_start.add(mem_len) };

    // u-boot places the device tree at the very end of physical memory and since we don't want to overwrite it,
    // we fake mem_end to end before it
    // mem_end = min(mem_end, args.phys_fdt_addr.cast_mut());   // TODO Re-add this

    // create an allocator to allocate essential data structures from the end of usable memory
    log::debug!(
        "creating allocator for general purpose memory start = {:p} end = {:p} (len = {} bytes)",
        mem_start,
        mem_end,
        mem_end as usize - mem_start as usize
    );
    let mut allocator = unsafe { BackwardBumpingAllocator::<'static>::new_raw(mem_start, mem_end) };

    // allocate a root PageTable for the initial kernel execution environment
    let root_table = unsafe {
        PageTable::empty(&mut allocator)
            .expect("Could not setup root PageTable")
            .as_mut()
            .unwrap()
    };
    log::debug!("root_table addr: {:p}", root_table);

    // load the kernel ELF file
    let mut kernel_loader = KernelLoader::new(allocator, root_table);
    let binary =
        ElfBinary::new(args.get_kernel_bin()).expect("Could not load kernel as elf object");
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
    let mut phys_dev_tree =
        BumpBox::new_uninit_slice_with_alignment(device_tree.buf().len(), 8, &allocator).unwrap();
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
