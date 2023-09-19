mod assign_ipc_buffer;
mod debug_log;
mod debug_putc;
mod identify;
mod map_page;
mod derive_from_mem;
mod task_assign_cspace;
mod task_assign_vspace;

use crate::caps::Capability;
use crate::sched::Schedule;
use crate::syscalls::assign_ipc_buffer::sys_assign_ipc_buffer;
use crate::syscalls::debug_log::sys_debug_log;
use crate::syscalls::debug_putc::sys_debug_putc;
use crate::syscalls::identify::sys_identify;
use crate::syscalls::map_page::sys_map_page;
use derivation_tree::tree::CursorRefMut;
use syscall_abi::assign_ipc_buffer::{AssignIpcBuffer, AssignIpcBufferArgs};
use syscall_abi::debug_log::{DebugLog, DebugLogArgs};
use syscall_abi::debug_putc::{DebugPutc, DebugPutcArgs};
use syscall_abi::generic_return::GenericReturn;
use syscall_abi::identify::{Identify, IdentifyArgs};
use syscall_abi::map_page::{MapPage, MapPageArgs};
use syscall_abi::*;
use syscall_abi::derive_from_mem::{DeriveFromMem, DeriveFromMemArgs};
use syscall_abi::task_assign_cspace::{TaskAssignCSpace, TaskAssignCSpaceArgs};
use syscall_abi::task_assign_vspace::{TaskAssignVSpace, TaskAssignVSpaceArgs};
use crate::syscalls::derive_from_mem::sys_derive_from_mem;
use crate::syscalls::task_assign_cspace::sys_task_assign_cspace;
use crate::syscalls::task_assign_vspace::sys_task_assign_vspace;

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
pub fn handle_syscall(task: &mut CursorRefMut<'_, '_, Capability>) -> Schedule {
    let (syscall_no, args) = {
        let mut task_state = task.get_inner_task_mut().unwrap().state.borrow_mut();
        let tf = &mut task_state.frame;
        let syscall_no = tf.get_syscall_number();
        let args: RawSyscallArgs = tf.get_syscall_args().try_into().unwrap();
        (syscall_no, args)
    };

    // actually handle the specific syscall
    let res: RawSyscallReturn = match syscall_no {
        DebugPutc::SYSCALL_NO => sys_debug_putc(DebugPutcArgs::try_from(args).unwrap())
            .unwrap()
            .into(),

        DebugLog::SYSCALL_NO => sys_debug_log(DebugLogArgs::try_from(args).unwrap())
            .unwrap()
            .into(),

        Identify::SYSCALL_NO => {
            log::debug!(
                "handling identify syscall with args {:x?}",
                IdentifyArgs::try_from(args).unwrap()
            );
            let result = sys_identify(task, IdentifyArgs::try_from(args).unwrap());
            log::debug!("identify syscall result is {:x?}", result);
            result.into()
        }

        DeriveFromMem::SYSCALL_NO => {
            log::debug!("handling derive_from_mem syscall with args {:?}", DeriveFromMemArgs::from(args));
            let result = sys_derive_from_mem(task, DeriveFromMemArgs::from(args));
            log::debug!("derive_from_mem result is {:?}", result);
            result.into()
        },

        MapPage::SYSCALL_NO => {
            log::debug!(
                "handling map_page syscall with args {:?}",
                MapPageArgs::try_from(args).unwrap()
            );
            let result = sys_map_page(task, MapPageArgs::try_from(args).unwrap());
            log::debug!("map_page syscall result is {:?}", result);
            result.into()
        }

        AssignIpcBuffer::SYSCALL_NO => {
            log::debug!(
                "handling assign_ipc_buffer syscall with args {:?}",
                AssignIpcBufferArgs::try_from(args).unwrap()
            );
            let result = sys_assign_ipc_buffer(task, AssignIpcBufferArgs::try_from(args).unwrap());
            log::debug!("assign_ipc_buffer syscall result is {:?}", result);
            result.into()
        }

        TaskAssignCSpace::SYSCALL_NO => {
            log::debug!(
                "handling task_assign_cspace syscall with args {:?}",
                TaskAssignCSpaceArgs::try_from(args).unwrap()
            );
            let result = sys_task_assign_cspace(task, TaskAssignCSpaceArgs::try_from(args).unwrap());
            log::debug!("task_assign_cspace syscall result is {:?}", result);
            result.into()
        },

        TaskAssignVSpace::SYSCALL_NO => {
            log::debug!(
                "handling task_assign_vspace syscall with args {:?}",
                TaskAssignVSpaceArgs::try_from(args).unwrap()
            );
            let result = sys_task_assign_vspace(task, TaskAssignVSpaceArgs::try_from(args).unwrap());
            log::debug!("task_assign_vspace syscall result is {:?}", result);
            result.into()
        },

        no => {
            log::warn!(
                "received unknown syscall {} with args {:x?}",
                syscall_no,
                args
            );
            GenericReturn::UnsupportedSyscall.into()
        }
    };

    // write the result back to userspace
    let [a0, a1]: RawSyscallReturn = res.into();
    let mut task_state = task.get_inner_task_mut().unwrap().state.borrow_mut();
    let tf = &mut task_state.frame;
    tf.write_syscall_result(a0, a1);
    Schedule::Keep
}
