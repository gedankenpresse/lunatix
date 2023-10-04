#![no_std]
#![no_main]

use allocators::Box;
use core::arch::asm;
use core::panic::PanicInfo;
use derivation_tree::tree::DerivationTree;
use kernel::caps::task::TaskExecutionState;
use kernel::caps::{Capability, IrqControlIface, KernelAlloc, NotificationIface};
use kernel::sched::Schedule;
use kernel::syscalls::SyscallContext;
use kernel::{syscalls, KERNEL_ALLOCATOR, KERNEL_ROOT_PT};
use libkernel::arch;
use libkernel::log::KernelLogger;
use libkernel::mem::ptrs::{MappedConstPtr, PhysConstPtr, PhysMutPtr};
use libkernel::println;
use log::Level;
use riscv::cpu::{Exception, Interrupt, TrapEvent};
use riscv::pt::PageTable;
use riscv::timer::set_next_timer;

static LOGGER: KernelLogger = KernelLogger::new(Level::Debug);

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    // print panic message
    println!("🚨 Kernel Panic! 😱  {}", info);

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
    _dtb: *const u8,
    phys_mem_start: PhysMutPtr<u8>,
    phys_mem_end: PhysMutPtr<u8>,
) {
    use kernel::init::*;

    unsafe { KERNEL_ALLOCATOR = Some(init_alloc(phys_mem_start, phys_mem_end)) };
    let allocator: &'static KernelAlloc = unsafe { (&mut KERNEL_ALLOCATOR).as_mut().unwrap() };

    // parse device tree from bootloader
    // let device_tree = unsafe { DevTree::from_raw_pointer(dtb).unwrap() };

    let kernel_root_pt = init_kernel_pagetable();
    unsafe { KERNEL_ROOT_PT = MappedConstPtr::from(kernel_root_pt as *const PageTable).as_direct() }

    let plic = unsafe {
        use kernel::arch_specific::plic::*;
        let plic = init_plic(PhysMutPtr::from(0xc000000 as *mut PLIC).as_mapped().raw());
        plic
    };

    let mut uart = unsafe {
        uart_driver::Uart::from_ptr(
            PhysMutPtr::from(0x10000000 as *mut uart_driver::MmUart)
                .as_mapped()
                .raw(),
        )
    };
    plic.set_threshold(1, 1);
    uart.enable_rx_interrupts();

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

    // set the context object for the following main loop
    let mut ctx = SyscallContext { plic };

    unsafe {
        set_return_to_user();
    };
    log::info!("🚀 launching init");
    let mut active_cursor = derivation_tree.get_node(&mut *init_caps.init_task).unwrap();
    let mut schedule = Schedule::RunInit;
    loop {
        match schedule {
            Schedule::RunInit => {
                active_cursor = derivation_tree.get_node(&mut *init_caps.init_task).unwrap();

                {
                    let handle = active_cursor.get_shared().unwrap();
                    if handle
                        .get_inner_task()
                        .unwrap()
                        .state
                        .borrow()
                        .execution_state
                        == TaskExecutionState::Waiting
                    {
                        unsafe { asm!("wfi") };
                    }
                }

                prepare_task(&mut active_cursor.get_exclusive().unwrap());
            }
            Schedule::Keep => {}
            Schedule::RunTask(task_cap) => {
                active_cursor = derivation_tree.get_node(task_cap).unwrap();
                prepare_task(&mut active_cursor.get_exclusive().unwrap());
            }
            Schedule::Stop => break,
        };

        let mut active_task = active_cursor.get_exclusive().unwrap();
        let trap_info = yield_to_task(&mut active_task);

        match trap_info.cause {
            TrapEvent::Exception(Exception::EnvCallFromUMode) => {
                {
                    let mut task_state =
                        active_task.get_inner_task_mut().unwrap().state.borrow_mut();
                    let tf = &mut task_state.frame;
                    tf.start_pc = trap_info.epc + 4;
                };
                schedule = syscalls::handle_syscall(&mut active_task, &mut ctx);
            }
            TrapEvent::Interrupt(Interrupt::SupervisorTimerInterrupt) => {
                log::trace!("⏰");
                const MILLI: u64 = 10_000; // 10_000 * time_base (100 nanos) ;
                set_next_timer(100 * MILLI).expect("Could not set new timer interrupt");
                {
                    let mut task_state =
                        active_task.get_inner_task_mut().unwrap().state.borrow_mut();
                    let tf = &mut task_state.frame;
                    tf.start_pc = trap_info.epc;
                };

                schedule = Schedule::RunInit;
            }
            TrapEvent::Interrupt(Interrupt::SupervisorExternalInterrupt) => {
                let claim = ctx.plic.claim_next(1).expect("no claim available");

                if let Some(notification) =
                    IrqControlIface.get_irq_notification(&mut init_caps.irq_control, claim)
                {
                    log::debug!("triggering notification for irq 0x{:x}", claim);
                    NotificationIface.notify(&notification.borrow());
                }

                // TODO claim the claim the notification

                // assert!(uart.has_rx());
                // let c = unsafe { uart.read_data() } as char;
                // if c == ':' {
                //     panic!("panic test")
                // }
                // log::debug!("✍️  {c}");
                // ctx.plic.complete(1, claim);
                //
                // {
                //     let mut task_state =
                //         active_task.get_inner_task_mut().unwrap().state.borrow_mut();
                //     let tf = &mut task_state.frame;
                //     tf.start_pc = trap_info.epc;
                // };

                schedule = Schedule::Keep;
            }
            _ => {
                println!("Interrupt!: Cause: {:#x?}", trap_info);
                panic!("interrupt type is not handled yet");
            }
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
