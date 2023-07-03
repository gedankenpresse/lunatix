use crate::{InitCaps, INIT_CAPS};
use core::ops::{Deref, DerefMut};
use libkernel::arch::cpu::{Exception, Interrupt, TrapEvent};
use libkernel::arch::timers::set_next_timer;
use libkernel::arch::trap::TrapFrame;
use libkernel::println;

#[no_mangle]
fn handle_trap(tf: &mut TrapFrame) -> &mut TrapFrame {
    let last_trap = tf.last_trap.as_ref().unwrap();

    match last_trap.cause {
        TrapEvent::Exception(Exception::EnvCallFromUMode) => {
            println!(
                "got call from user: {}",
                tf.general_purpose_regs[10] as u8 as char
            );
            tf.start_pc = last_trap.epc + 4;
            tf
        }
        TrapEvent::Interrupt(Interrupt::SupervisorTimerInterrupt) => {
            log::debug!("timer interrupt triggered. switching back to init task");
            set_next_timer(10_000_000).expect("Could not set new timer interrupt");
            tf.start_pc = last_trap.epc;

            // get a handle to the init caps but also drop the guard so that it can be acquired later too
            let mut guard = INIT_CAPS
                .try_lock()
                .expect("Could not acquire lock for INIT_CAPS");
            let init_caps = guard.deref_mut() as *mut InitCaps;
            drop(guard);
            let init_caps = unsafe { &mut *init_caps };

            unsafe {
                &mut (*(init_caps
                    .init_task
                    .cap
                    .get_task_mut()
                    .unwrap()
                    .content
                    .state))
                    .frame
            }
        }
        _ => {
            println!("Interrupt!: Cause: {:#x?}", last_trap);
            panic!("interrupt type is not handled yet");
        }
    }
}
