mod copy;
mod destroy;
mod identify;
mod r#yield;
mod yield_to;

mod debug;

mod call;
mod exit;
mod handler_trait;
mod ipc;
mod send;
mod system_reset;
mod utils;
mod wait_on;

use crate::caps::Capability;
use crate::sched::Schedule;
use crate::syscalls::debug::sys_debug_log;
use crate::syscalls::debug::sys_debug_putc;
use crate::syscalls::identify::IdentifyHandler;
use crate::syscalls::r#yield::sys_yield;
use crate::syscalls::system_reset::sys_system_reset;
use crate::syscalls::wait_on::sys_wait_on;
use crate::syscalls::yield_to::sys_yield_to;
use crate::KernelContext;
use derivation_tree::tree::CursorRefMut;
use riscv::trap::TrapInfo;
use syscall_abi::debug::{DebugLog, DebugLogArgs};
use syscall_abi::debug::{DebugPutc, DebugPutcArgs};
use syscall_abi::identify::{Identify, IdentifyArgs};
use syscall_abi::r#yield::Yield;
use syscall_abi::system_reset::{SystemReset, SystemResetArgs};

use crate::syscalls::exit::sys_exit;
use crate::syscalls::handler_trait::SyscallHandler;
use syscall_abi::call::Call;
use syscall_abi::exit::Exit;
use syscall_abi::send::SendArgs;
use syscall_abi::wait_on::{WaitOn, WaitOnArgs};
use syscall_abi::yield_to::{YieldTo, YieldToArgs};
use syscall_abi::*;

pub(self) struct SyscallContext<'trap_info, 'cursor, 'cursor_handle, 'cursor_set> {
    pub task: &'cursor mut CursorRefMut<'cursor_handle, 'cursor_set, Capability>,
    pub trap_info: &'trap_info TrapInfo,
}

impl<'trap_info, 'cursor, 'cursor_handle, 'cursor_set>
    SyscallContext<'trap_info, 'cursor, 'cursor_handle, 'cursor_set>
{
    fn from(
        trap_info: &'trap_info TrapInfo,
        task: &'cursor mut CursorRefMut<'cursor_handle, 'cursor_set, Capability>,
    ) -> Self {
        Self { trap_info, task }
    }
}

pub(self) type HandlerReturn<Syscall> = (Schedule, <Syscall as SyscallBinding>::Return);

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
    trap_info: &TrapInfo,
    kernel_ctx: &mut KernelContext,
) -> Schedule {
    // extract syscall number and raw arguments from calling tasks registers
    let (syscall_no, raw_args) = {
        let mut task_state = task.get_inner_task_mut().unwrap().state.borrow_mut();
        let tf = &mut task_state.frame;
        let syscall_no = tf.get_syscall_number();
        let args: RawSyscallArgs = tf.get_syscall_args().try_into().unwrap();
        (syscall_no, args)
    };

    match syscall_no {
        // standardized syscalls
        Identify::SYSCALL_NO => handle_specific_syscall(
            IdentifyHandler,
            kernel_ctx,
            &mut SyscallContext::from(trap_info, task),
            raw_args,
        ),

        _ => {
            {
                // increase the tasks program counter
                let mut task_state = task.get_inner_task_mut().unwrap().state.borrow_mut();
                task_state.frame.start_pc = trap_info.epc + 4;
            }

            // actually handle the specific syscall
            let (res, schedule): (RawSyscallReturn, Schedule) = match syscall_no {
                DebugPutc::SYSCALL_NO => (
                    sys_debug_putc(DebugPutcArgs::try_from(raw_args).unwrap()).into_response(),
                    Schedule::Keep,
                ),

                DebugLog::SYSCALL_NO => (
                    sys_debug_log(DebugLogArgs::try_from(raw_args).unwrap()).into_response(),
                    Schedule::Keep,
                ),

                // AssignIpcBuffer::SYSCALL_NO => {
                //     log::debug!(
                //         "handling assign_ipc_buffer syscall with args {:?}",
                //         AssignIpcBufferArgs::try_from(args).unwrap()
                //     );
                //     let result = sys_assign_ipc_buffer(task, AssignIpcBufferArgs::try_from(args).unwrap());
                //     log::debug!("assign_ipc_buffer syscall result is {:?}", result);
                //     (result.into_response(), Schedule::Keep)
                // }
                YieldTo::SYSCALL_NO => {
                    log::debug!(
                        "handling yield_to syscall with args {:?}",
                        YieldToArgs::from(raw_args)
                    );
                    let (result, schedule) = sys_yield_to(task, YieldToArgs::from(raw_args));
                    log::debug!("yield_to result is {:?}", result);
                    (result.into_response(), schedule)
                }

                Yield::SYSCALL_NO => {
                    log::debug!("handling yield syscall",);
                    let (result, schedule) = sys_yield(NoValue);
                    log::debug!("yield result is {:?}", result);
                    (result.into_response(), schedule)
                }

                WaitOn::SYSCALL_NO => {
                    let args = WaitOnArgs::from(raw_args);
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
                    let args = SystemResetArgs::from(raw_args);
                    log::debug!("handling system_reset syscall with args {:?}", args);
                    sys_system_reset(args);
                }

                /* SEND SYSCALL */
                syscall_abi::send::Send::SYSCALL_NO => {
                    let args = SendArgs::from(raw_args);
                    log::debug!("handling send syscall with args {:?}", args);
                    let result = send::sys_send(kernel_ctx, task, args);
                    log::debug!("send result is {:?}", result);
                    (result.into_response(), Schedule::Keep)
                }

                /* DESTROY SYSCALL */
                // TODO Update syscall_abi to include destroy
                19 => {
                    let result = destroy::sys_destroy(kernel_ctx, task, &raw_args);
                    let response = match result {
                        Ok(()) => Ok(NoValue),
                        Err(e) => Err(e),
                    };
                    (response.into_response(), Schedule::Keep)
                }

                /* COPY SYSCALL */
                // TODO Update syscall_abi to include copy
                20 => {
                    let result = copy::sys_copy(kernel_ctx, task, &raw_args);
                    let response = match result {
                        Ok(()) => Ok(NoValue),
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

                Call::SYSCALL_NO => {
                    let args = <Call as SyscallBinding>::CallArgs::from(raw_args);
                    log::debug!("handling call syscall with args {:?}", args);
                    let result = call::sys_call(kernel_ctx, task, args);
                    log::debug!("call result is {:?}", result);
                    (result.into_response(), Schedule::Keep)
                }

                _no => {
                    log::warn!(
                        "received unknown syscall {} with args {:x?}",
                        syscall_no,
                        raw_args
                    );
                    (
                        [SyscallError::UnknownSyscall as usize, 0, 0, 0, 0, 0, 0, 0],
                        Schedule::Keep,
                    )
                }
            };

            // write the result back to userspace
            if res[0] != SyscallError::WouldBlock as usize {
                let mut task_state = task.get_inner_task_mut().unwrap().state.borrow_mut();
                let tf = &mut task_state.frame;
                tf.write_syscall_return(res);
            }

            schedule
        }
    }
}

fn handle_specific_syscall<Handler: SyscallHandler>(
    mut handler: Handler,
    kernel_ctx: &mut KernelContext,
    syscall_ctx: &mut SyscallContext<'_, '_, '_, '_>,
    raw_args: RawSyscallArgs,
) -> Schedule {
    // parse syscall arguments
    let args = <Handler::Syscall as SyscallBinding>::CallArgs::try_from(raw_args)
        .unwrap_or_else(|_| panic!("could not decode syscall args"));

    // execute the handler
    log::debug!(
        "handling {} syscall with args {:x?}",
        core::any::type_name::<Handler::Syscall>(),
        args
    );
    let (schedule, result) = handler.handle(kernel_ctx, syscall_ctx, args);
    log::debug!(
        "{} syscall result is {:x?} with new schedule {:?}",
        core::any::type_name::<Handler::Syscall>(),
        result,
        schedule
    );

    // write the result back to userspace
    let mut task_state = syscall_ctx
        .task
        .get_inner_task()
        .unwrap()
        .state
        .borrow_mut();
    task_state
        .frame
        .write_syscall_return(result.into_response());

    // increase the tasks program counter
    task_state.frame.start_pc = syscall_ctx.trap_info.epc + 4;

    schedule
}
