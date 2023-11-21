#![no_std]
#![no_main]

use crate::caps::task::TaskExecutionState;
use crate::caps::{Capability, IrqControlIface, KernelAlloc, NotificationIface};
use crate::init::InitCaps;
use crate::sched::Schedule;
use allocators::Box;
use core::arch::asm;
use core::panic::PanicInfo;
use derivation_tree::tree::DerivationTree;
use klog::KernelLogger;
use log::Level;
use riscv::cpu::{Exception, Interrupt, TrapEvent};
use riscv::mem::ptrs::{PhysConstPtr, PhysMutPtr};
use riscv::mem::VIRT_MEM_KERNEL_START;
use riscv::pt::PageTable;
use riscv::timer::set_next_timer;
use riscv::trap::{set_kernel_trap_handler, set_user_trap_handler};

mod caps;
mod devtree;
mod init;
mod sched;
mod syscalls;
mod virtmem;

#[macro_use]
extern crate klog;

#[cfg(target_arch = "riscv64")]
#[path = "arch/riscv64imac/mod.rs"]
mod arch_specific;

static LOGGER: KernelLogger = KernelLogger::new(Level::Info);

/// A global static reference to the root PageTable which has only the kernel part mapped
pub static mut KERNEL_ROOT_PT: PhysConstPtr<PageTable> = PhysConstPtr::null();

pub static mut KERNEL_ALLOCATOR: Option<KernelAlloc> = None;

pub struct KernelContext {
    pub plic: &'static mut arch_specific::plic::PLIC,
}

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    // print panic message
    println!("🚨 Kernel Panic! 😱  {}", info);

    // shutdown the device
    riscv::shutdown()
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

    kernel_main(0, 0, phys_fdt, phys_mem_start, phys_mem_end);
    riscv::shutdown();
}

extern "C" fn kernel_main(
    _hartid: usize,
    _unused: usize,
    dtb: PhysConstPtr<u8>,
    phys_mem_start: PhysMutPtr<u8>,
    phys_mem_end: PhysMutPtr<u8>,
) {
    use crate::init::*;

    let allocator: &KernelAlloc = init_kernel_allocator(phys_mem_start, phys_mem_end);
    let dt = init_device_tree(dtb.as_mapped().into());
    init_kernel_root_pt();

    let plic = init_plic();

    let derivation_tree = init_derivation_tree(allocator);
    let mut init_caps = create_init_caps(&allocator, &derivation_tree, &dt);
    load_init_task(&derivation_tree, &mut init_caps, dtb);
    map_device_tree(
        init_caps
            .init_task
            .get_inner_task()
            .unwrap()
            .get_cspace()
            .get_shared()
            .unwrap()
            .get_inner_cspace()
            .unwrap()
            .slots[1]
            .borrow()
            .get_inner_memory()
            .unwrap(),
        init_caps
            .init_task
            .get_inner_task()
            .unwrap()
            .get_vspace()
            .get_shared()
            .unwrap()
            .get_inner_vspace()
            .unwrap(),
        &dt,
    );

    prepare_userspace_handoff();

    kernel_loop(derivation_tree, init_caps, &mut KernelContext { plic });
}

fn kernel_loop(
    derivation_tree: Box<DerivationTree<Capability>>,
    mut init_caps: InitCaps,
    ctx: &mut KernelContext,
) {
    use crate::init::{prepare_task, yield_to_task};
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
                schedule = syscalls::handle_syscall(&mut active_task, &trap_info, ctx);
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
