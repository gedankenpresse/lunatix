#![no_std]
#![no_main]

use allocators::Box;
use core::arch::asm;
use core::panic::PanicInfo;
use derivation_tree::tree::DerivationTree;
use kernel::caps::task::TaskExecutionState;
use kernel::caps::{Capability, IrqControlIface, KernelAlloc, NotificationIface};
use kernel::sched::Schedule;
use kernel::{syscalls, InitCaps, SyscallContext};
use libkernel::arch;
use libkernel::log::KernelLogger;
use libkernel::mem::ptrs::{PhysConstPtr, PhysMutPtr};
use libkernel::println;
use log::Level;
use riscv::cpu::{Exception, Interrupt, TrapEvent};
use riscv::timer::set_next_timer;
use riscv::trap::{set_kernel_trap_handler, set_user_trap_handler};

static LOGGER: KernelLogger = KernelLogger::new(Level::Info);

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
    dtb: *const u8,
    phys_mem_start: PhysMutPtr<u8>,
    phys_mem_end: PhysMutPtr<u8>,
) {
    use kernel::init::*;

    let allocator: &KernelAlloc = init_kernel_allocator(phys_mem_start, phys_mem_end);
    let dt = init_device_tree(dtb);
    let mut external_device_buf: [_; 16] = core::array::from_fn(|_| None);
    let external_devices = kernel::devtree::get_external_devices(&dt, &mut external_device_buf);

    init_kernel_root_pt();

    let plic = init_plic();

    let derivation_tree = init_derivation_tree(allocator);
    let mut init_caps = create_init_caps(&allocator, &derivation_tree, &dt);
    load_init_task(&derivation_tree, &mut init_caps);

    prepare_userspace_handoff();

    kernel_loop(derivation_tree, init_caps, &mut SyscallContext { plic });
}

fn kernel_loop(
    derivation_tree: Box<DerivationTree<Capability>>,
    mut init_caps: InitCaps,
    ctx: &mut SyscallContext,
) {
    use kernel::init::{prepare_task, yield_to_task};
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
        unsafe {
            set_user_trap_handler();
        }
        let trap_info = yield_to_task(&mut active_task);
        unsafe {
            set_kernel_trap_handler();
        }

        match trap_info.cause {
            TrapEvent::Exception(Exception::EnvCallFromUMode) => {
                {
                    let mut task_state =
                        active_task.get_inner_task_mut().unwrap().state.borrow_mut();
                    let tf = &mut task_state.frame;
                    tf.start_pc = trap_info.epc + 4;
                };
                schedule = syscalls::handle_syscall(&mut active_task, ctx);
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

                {
                    let mut task_state = active_task.get_inner_task().unwrap().state.borrow_mut();
                    task_state.frame.start_pc = trap_info.epc;
                }

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
