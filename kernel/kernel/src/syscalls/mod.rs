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
use crate::syscalls::debug::{DebugLogHandler, DebugPutcHandler};
use crate::syscalls::identify::IdentifyHandler;
use crate::syscalls::r#yield::YieldHandler;
use crate::syscalls::system_reset::SystemResetHandler;
use crate::syscalls::wait_on::WaitOnHandler;
use crate::syscalls::yield_to::YieldToHandler;
use crate::KernelContext;
use derivation_tree::tree::CursorRefMut;
use riscv::trap::TrapInfo;
use syscall_abi::debug::DebugLog;
use syscall_abi::debug::DebugPutc;
use syscall_abi::identify::Identify;
use syscall_abi::r#yield::Yield;
use syscall_abi::system_reset::SystemReset;

use crate::syscalls::call::CallHandler;
use crate::syscalls::copy::CopyHandler;
use crate::syscalls::destroy::DestroyHandler;
use crate::syscalls::exit::ExitHandler;
use crate::syscalls::handler_trait::{RawSyscallHandler, SyscallHandler};
use crate::syscalls::send::SendHandler;
use syscall_abi::call::Call;
use syscall_abi::destroy::Destroy;
use syscall_abi::exit::Exit;
use syscall_abi::wait_on::WaitOn;
use syscall_abi::yield_to::YieldTo;
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

    let mut syscall_ctx = SyscallContext::from(trap_info, task);

    match syscall_no {
        // standardized syscalls
        DebugPutc::SYSCALL_NO => {
            wrap_handler(DebugPutcHandler, kernel_ctx, &mut syscall_ctx, raw_args)
        }
        DebugLog::SYSCALL_NO => {
            wrap_handler(DebugLogHandler, kernel_ctx, &mut syscall_ctx, raw_args)
        }

        Identify::SYSCALL_NO => {
            wrap_handler(IdentifyHandler, kernel_ctx, &mut syscall_ctx, raw_args)
        }

        YieldTo::SYSCALL_NO => wrap_handler(YieldToHandler, kernel_ctx, &mut syscall_ctx, raw_args),

        Yield::SYSCALL_NO => wrap_handler(YieldHandler, kernel_ctx, &mut syscall_ctx, raw_args),

        SystemReset::SYSCALL_NO => {
            wrap_handler(SystemResetHandler, kernel_ctx, &mut syscall_ctx, raw_args)
        }

        syscall_abi::send::Send::SYSCALL_NO => {
            wrap_handler(SendHandler, kernel_ctx, &mut syscall_ctx, raw_args)
        }

        Exit::SYSCALL_NO => wrap_handler(ExitHandler, kernel_ctx, &mut syscall_ctx, raw_args),

        Call::SYSCALL_NO => wrap_handler(CallHandler, kernel_ctx, &mut syscall_ctx, raw_args),

        Destroy::SYSCALL_NO => wrap_handler(DestroyHandler, kernel_ctx, &mut syscall_ctx, raw_args),

        syscall_abi::copy::Copy::SYSCALL_NO => {
            wrap_handler(CopyHandler, kernel_ctx, &mut syscall_ctx, raw_args)
        }

        // raw syscalls
        WaitOn::SYSCALL_NO => WaitOnHandler.handle(kernel_ctx, &mut syscall_ctx, raw_args),

        _ => {
            log::warn!(
                "received unknown syscall {} with args {:x?}",
                syscall_no,
                raw_args
            );

            // write error into task
            let mut task_state = task.get_inner_task().unwrap().state.borrow_mut();
            task_state.frame.write_syscall_return(
                SyscallResult::<NoValue>::Err(SyscallError::UnknownSyscall).into_response(),
            );
            task_state.frame.start_pc = trap_info.epc + 4;

            Schedule::Keep
        }
    }
}

fn wrap_handler<Handler: SyscallHandler>(
    mut handler: Handler,
    kernel_ctx: &mut KernelContext,
    syscall_ctx: &mut SyscallContext<'_, '_, '_, '_>,
    raw_args: RawSyscallArgs,
) -> Schedule {
    // parse syscall arguments
    let args = <Handler::Syscall as SyscallBinding>::CallArgs::try_from(raw_args)
        .unwrap_or_else(|_| panic!("could not decode syscall args"));

    // execute the handler
    log::trace!(
        "handling {} syscall with args {:x?}",
        core::any::type_name::<Handler::Syscall>(),
        args
    );
    let (schedule, result) = handler.handle(kernel_ctx, syscall_ctx, args);
    log::trace!(
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
