mod assign_ipc_buffer;
mod copy;
mod destroy;
mod identify;
mod r#yield;
mod yield_to;

mod asid_control;
mod debug;
mod irq;
mod mem;
mod page;
mod task;

mod devmem;
mod exit;
mod send;
mod system_reset;
mod utils;
mod wait_on;

use crate::caps::Capability;
use crate::sched::Schedule;
use crate::syscalls::assign_ipc_buffer::sys_assign_ipc_buffer;
use crate::syscalls::debug::sys_debug_log;
use crate::syscalls::debug::sys_debug_putc;
use crate::syscalls::identify::sys_identify;
use crate::syscalls::r#yield::sys_yield;
use crate::syscalls::system_reset::sys_system_reset;
use crate::syscalls::wait_on::sys_wait_on;
use crate::syscalls::yield_to::sys_yield_to;
use crate::SyscallContext;
use derivation_tree::tree::CursorRefMut;
use syscall_abi::assign_ipc_buffer::{AssignIpcBuffer, AssignIpcBufferArgs};
use syscall_abi::debug::{DebugLog, DebugLogArgs};
use syscall_abi::debug::{DebugPutc, DebugPutcArgs};
use syscall_abi::identify::{Identify, IdentifyArgs};
use syscall_abi::r#yield::{Yield, YieldArgs};
use syscall_abi::system_reset::{SystemReset, SystemResetArgs};

use crate::syscalls::exit::sys_exit;
use syscall_abi::exit::Exit;
use syscall_abi::send::SendArgs;
use syscall_abi::wait_on::{WaitOn, WaitOnArgs};
use syscall_abi::yield_to::{YieldTo, YieldToArgs};
use syscall_abi::*;

#[derive(Debug)]
#[repr(usize)]
pub enum SyscallError {
    InvalidCAddr = 1,
    NoMem = 2,
    OccupiedSlot = 3,
    InvalidCap = 4,
    InvalidArg = 6,
    AliasingCSlot = 7,
    InvalidReturn = 8,
    Unsupported = 9,
}

/// Handle a syscall from userspace.
///
/// The function expects the syscall information to be present in the passed TrapFrames registers.
///
/// After the syscall has been handled, this function returns another TrapFrame which should now be
/// executed on the CPU.
/// It might be the same as `tf` but might also not be.
#[inline(always)]
pub fn handle_syscall(
    task: &mut CursorRefMut<'_, '_, Capability>,
    ctx: &mut SyscallContext,
) -> Schedule {
    let (syscall_no, args) = {
        let mut task_state = task.get_inner_task_mut().unwrap().state.borrow_mut();
        let tf = &mut task_state.frame;
        let syscall_no = tf.get_syscall_number();
        let args: RawSyscallArgs = tf.get_syscall_args().try_into().unwrap();
        (syscall_no, args)
    };

    // actually handle the specific syscall
    let (res, schedule): (RawSyscallReturn, Schedule) = match syscall_no {
        DebugPutc::SYSCALL_NO => (
            sys_debug_putc(DebugPutcArgs::try_from(args).unwrap()).into_response(),
            Schedule::Keep,
        ),

        DebugLog::SYSCALL_NO => (
            sys_debug_log(DebugLogArgs::try_from(args).unwrap()).into_response(),
            Schedule::Keep,
        ),

        Identify::SYSCALL_NO => {
            log::debug!(
                "handling identify syscall with args {:x?}",
                IdentifyArgs::try_from(args).unwrap()
            );
            let result = sys_identify(task, IdentifyArgs::try_from(args).unwrap());
            log::debug!("identify syscall result is {:x?}", result);
            (result.into_response(), Schedule::Keep)
        }

        AssignIpcBuffer::SYSCALL_NO => {
            log::debug!(
                "handling assign_ipc_buffer syscall with args {:?}",
                AssignIpcBufferArgs::try_from(args).unwrap()
            );
            let result = sys_assign_ipc_buffer(task, AssignIpcBufferArgs::try_from(args).unwrap());
            log::debug!("assign_ipc_buffer syscall result is {:?}", result);
            (result.into_response(), Schedule::Keep)
        }

        YieldTo::SYSCALL_NO => {
            log::debug!(
                "handling yield_to syscall with args {:?}",
                YieldToArgs::from(args)
            );
            let (result, schedule) = sys_yield_to(task, YieldToArgs::from(args));
            log::debug!("yield_to result is {:?}", result);
            (result.into_response(), schedule)
        }

        Yield::SYSCALL_NO => {
            log::debug!(
                "handling yield syscall with args {:?}",
                YieldArgs::from(args)
            );
            let (result, schedule) = sys_yield(YieldArgs::from(args));
            log::debug!("yield result is {:?}", result);
            (result.into_response(), schedule)
        }

        WaitOn::SYSCALL_NO => {
            let args = WaitOnArgs::from(args);
            log::debug!("handling wait_on syscall with args {:?}", args);
            let (result, schedule) = sys_wait_on(task, args);
            log::debug!(
                "wait_on result is {:?} with schedule {:?}",
                result,
                schedule
            );
            (result.into_response(), schedule)
        }

        SystemReset::SYSCALL_NO => {
            let args = SystemResetArgs::from(args);
            log::debug!("handling system_reset syscall with args {:?}", args);
            sys_system_reset(args);
        }

        /* SEND SYSCALL */
        syscall_abi::send::Send::SYSCALL_NO => {
            let args = SendArgs::from(args);
            log::debug!("handling send syscall with args {:?}", args);
            let result = send::sys_send(ctx, task, args);
            log::debug!("send result is {:?}", result);
            let response = match result {
                Ok(()) => Ok(NoValue),
                Err(e) => Err(e),
            };
            (response.into_response(), Schedule::Keep)
        }

        /* DESTROY SYSCALL */
        19 => {
            let result = destroy::sys_destroy(ctx, task, &args);
            let response = match result {
                Ok(()) => Ok(NoValue),
                Err(e) => Err(e),
            };
            (response.into_response(), Schedule::Keep)
        }

        /* COPY SYSCALL */
        20 => {
            let result = copy::sys_copy(ctx, task, &args);
            let response = match result {
                Ok(()) => Ok(NoValue),
                Err(e) => Err(e),
            };
            (response.into_response(), Schedule::Keep)
        }
        /* Get Page PADDR SYSCALL */
        21 => {
            let result = page::page_paddr(ctx, task, &args);
            let response = match result {
                Ok(addr) => Ok(addr),
                Err(e) => Err(e),
            };
            (response.into_response(), Schedule::Keep)
        }

        Exit::SYSCALL_NO => {
            log::debug!("handling exit syscall");
            let task = task.get_inner_task().unwrap();
            sys_exit(task);
            (Default::default(), Schedule::RunInit)
        }

        no => {
            log::warn!(
                "received unknown syscall {} with args {:x?}",
                syscall_no,
                args
            );
            ([Error::UnknownSyscall as usize, 0], Schedule::Keep)
        }
    };

    // write the result back to userspace
    if res[0] != Error::WouldBlock as usize {
        let [a0, a1] = res;
        let mut task_state = task.get_inner_task_mut().unwrap().state.borrow_mut();
        let tf = &mut task_state.frame;
        tf.write_syscall_result(a0, a1);
    }
    schedule
}
