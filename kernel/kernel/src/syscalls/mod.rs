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
mod receive;
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
use syscall_abi::receive::Receive;
use syscall_abi::system_reset::SystemReset;

use crate::syscalls::call::CallHandler;
use crate::syscalls::copy::CopyHandler;
use crate::syscalls::destroy::DestroyHandler;
use crate::syscalls::exit::ExitHandler;
use crate::syscalls::handler_trait::RawSyscallHandler;
use crate::syscalls::send::SendHandler;
use syscall_abi::call::Call;
use syscall_abi::destroy::Destroy;
use syscall_abi::exit::Exit;
use syscall_abi::wait_on::WaitOn;
use syscall_abi::yield_to::YieldTo;
use syscall_abi::*;

use self::receive::ReceiveHandler;

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
        let args: RawSyscallArgs = tf.get_syscall_args_mut().try_into().unwrap();
        (syscall_no, args)
    };

    let mut syscall_ctx = SyscallContext::from(trap_info, task);

    match syscall_no {
        // handle syscalls
        DebugPutc::SYSCALL_NO => {
            DebugPutcHandler.handle_raw(kernel_ctx, &mut syscall_ctx, raw_args)
        }
        DebugLog::SYSCALL_NO => DebugLogHandler.handle_raw(kernel_ctx, &mut syscall_ctx, raw_args),
        Identify::SYSCALL_NO => IdentifyHandler.handle_raw(kernel_ctx, &mut syscall_ctx, raw_args),
        YieldTo::SYSCALL_NO => YieldToHandler.handle_raw(kernel_ctx, &mut syscall_ctx, raw_args),
        Yield::SYSCALL_NO => YieldHandler.handle_raw(kernel_ctx, &mut syscall_ctx, raw_args),
        SystemReset::SYSCALL_NO => {
            SystemResetHandler.handle_raw(kernel_ctx, &mut syscall_ctx, raw_args)
        }
        syscall_abi::send::Send::SYSCALL_NO => {
            SendHandler.handle_raw(kernel_ctx, &mut syscall_ctx, raw_args)
        }
        Receive::SYSCALL_NO => ReceiveHandler.handle_raw(kernel_ctx, &mut syscall_ctx, raw_args),
        Exit::SYSCALL_NO => ExitHandler.handle_raw(kernel_ctx, &mut syscall_ctx, raw_args),
        Call::SYSCALL_NO => CallHandler.handle_raw(kernel_ctx, &mut syscall_ctx, raw_args),
        Destroy::SYSCALL_NO => DestroyHandler.handle_raw(kernel_ctx, &mut syscall_ctx, raw_args),
        syscall_abi::copy::Copy::SYSCALL_NO => {
            CopyHandler.handle_raw(kernel_ctx, &mut syscall_ctx, raw_args)
        }
        WaitOn::SYSCALL_NO => WaitOnHandler.handle_raw(kernel_ctx, &mut syscall_ctx, raw_args),

        // handle an unknown syscall
        _ => handle_unknown_syscall(&mut syscall_ctx, syscall_no, raw_args),
    }
}

fn handle_unknown_syscall(
    ctx: &SyscallContext,
    syscall_no: usize,
    raw_args: RawSyscallArgs,
) -> Schedule {
    log::warn!(
        "received unknown syscall {} with args {:x?}",
        syscall_no,
        raw_args
    );

    // write error into task
    let mut task_state = ctx.task.get_inner_task().unwrap().state.borrow_mut();
    task_state.frame.write_syscall_return(
        SyscallResult::<NoValue>::Err(SyscallError::UnknownSyscall).into_response(),
    );
    task_state.frame.start_pc = ctx.trap_info.epc + 4;

    Schedule::Keep
}
