use crate::caps::Capability;
use crate::sched::Schedule;
use crate::syscalls;
use derivation_tree::tree::CursorRefMut;
use libkernel::println;
use riscv::cpu::{Exception, Interrupt, TrapEvent};
use riscv::timer::set_next_timer;
use riscv::trap::TrapInfo;

/// Handle a RISCV trap.
///
/// The function expects the trap to have been triggered in the context of the given TrapFrame `tf`.
///
/// After the trap has been handled, the function returns another TrapFrame which should now be
/// executed on the CPU.
/// It might be the same as `tf` but it might also not be.
#[no_mangle]
pub fn handle_trap(task: &mut CursorRefMut<'_, '_, Capability>, last_trap: TrapInfo) -> Schedule {
    match last_trap.cause {
        TrapEvent::Exception(Exception::EnvCallFromUMode) => {
            {
                let mut task_state = task.get_inner_task_mut().unwrap().state.borrow_mut();
                let tf = &mut task_state.frame;
                tf.start_pc = last_trap.epc + 4;
            };
            syscalls::handle_syscall(task)
        }
        TrapEvent::Interrupt(Interrupt::SupervisorTimerInterrupt) => {
            log::debug!("⏰ timer interrupt triggered. switching back to init task");
            set_next_timer(10_000_000).expect("Could not set new timer interrupt");
            {
                let mut task_state = task.get_inner_task_mut().unwrap().state.borrow_mut();
                let tf = &mut task_state.frame;
                tf.start_pc = last_trap.epc;
            };

            Schedule::RunInit
        }
        TrapEvent::Interrupt(Interrupt::SupervisorExternalInterrupt) => Schedule::Keep,
        _ => {
            println!("Interrupt!: Cause: {:#x?}", last_trap);
            panic!("interrupt type is not handled yet");
        }
    }
}
