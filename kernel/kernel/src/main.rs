#![no_std]
#![no_main]

use allocators::Box;
use core::panic::PanicInfo;
use derivation_tree::tree::DerivationTree;
use kernel::caps::{Capability, KernelAlloc};
use kernel::sched::Schedule;
use kernel::trap::handle_trap;
use kernel::{KERNEL_ALLOCATOR, KERNEL_ROOT_PT};
use libkernel::arch;
use libkernel::log::KernelLogger;
use libkernel::mem::ptrs::{MappedConstPtr, PhysConstPtr, PhysMutPtr};
use libkernel::println;
use log::Level;
use riscv::pt::PageTable;

static LOGGER: KernelLogger = KernelLogger::new(Level::Debug);

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    // print panic message
    println!("🚨🚨🚨 Kernel Panic 🚨🚨🚨\n  {}", info);

    // shutdown the device
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
    arch::shutdown();
}

extern "C" fn kernel_main(
    _hartid: usize,
    _unused: usize,
    dtb: *const u8,
    phys_mem_start: PhysMutPtr<u8>,
    phys_mem_end: PhysMutPtr<u8>,
) {
    use kernel::init::*;

    unsafe { KERNEL_ALLOCATOR = Some(init_alloc(phys_mem_start, phys_mem_end)) }
    let allocator: &'static KernelAlloc = unsafe { (&mut KERNEL_ALLOCATOR).as_mut().unwrap() };

    // parse device tree from bootloader
    // let device_tree = unsafe { DevTree::from_raw_pointer(dtb).unwrap() };

    let kernel_root_pt = init_kernel_pagetable();
    unsafe { KERNEL_ROOT_PT = MappedConstPtr::from(kernel_root_pt as *const PageTable).as_direct() }

    // fill the derivation tree with initially required capabilities
    let mut derivation_tree = Box::new_uninit(allocator).unwrap();
    let derivation_tree = unsafe {
        DerivationTree::init_with_root_value(&mut derivation_tree, Capability::empty());
        derivation_tree.assume_init()
    };
    let mut init_caps = create_init_caps(&allocator, &derivation_tree);

    // load the init binary
    {
        let mut mem_cap = derivation_tree.get_root_cursor().unwrap();
        let mut mem_cap = mem_cap.get_exclusive().unwrap();
        load_init_binary(&mut init_caps.init_task, &mut mem_cap)
    }

    log::debug!("enabling interrupts");
    riscv::timer::set_next_timer(0).unwrap();
    riscv::trap::enable_interrupts();

    unsafe {
        set_return_to_user();
    };
    log::info!("🚀 launching init");
    let mut active_cursor = derivation_tree.get_node(&mut *init_caps.init_task).unwrap();
    loop {
        let mut active_task = active_cursor.get_exclusive().unwrap();
        let trap_info = yield_to_task(&mut active_task);

        match handle_trap(&mut active_task, trap_info) {
            Schedule::RunInit => {
                drop(active_task);
                active_cursor = derivation_tree.get_node(&mut *init_caps.init_task).unwrap();
            }
            Schedule::Keep => {}
            Schedule::RunTask(_) => todo!(),
            Schedule::Stop => break,
        }
    }
}

/// Assert that all environment conditions under which the kernel expects to be started are met
#[cfg(target_arch = "riscv64")]
fn assert_start_expectations() {
    use libkernel::mem::VIRT_MEM_KERNEL_START;
    use riscv::cpu::*;
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
        Sie::read(),
        InterruptBits::empty(),
        "kernel was started with interrupts already enabled"
    );
}

#[cfg(target_arch = "x86_64")]
fn assert_start_expectations() {
    todo!()
}
