mod alloc_page;
mod debug_log;
mod debug_putc;
mod identify;

use crate::syscalls::alloc_page::sys_alloc_page;
use crate::syscalls::debug_log::sys_debug_log;
use crate::syscalls::debug_putc::sys_debug_putc;
use crate::syscalls::identify::sys_identify;
use riscv::trap::TrapFrame;
use syscall_abi::alloc_page::{AllocPage, AllocPageArgs};
use syscall_abi::debug_log::{DebugLog, DebugLogArgs};
use syscall_abi::debug_putc::{DebugPutc, DebugPutcArgs};
use syscall_abi::generic_return::GenericReturn;
use syscall_abi::identify::{Identify, IdentifyArgs};
use syscall_abi::*;

const SYS_DEBUG_LOG: usize = 0;
const SYS_DEBUG_PUTC: usize = 1;
const SYS_SEND: usize = 2;
const SYS_IDENTIFY: usize = 3;
const SYS_DESTROY: usize = 4;

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
pub(crate) fn handle_syscall(tf: &mut TrapFrame) -> &mut TrapFrame {
    let syscall_no = tf.get_syscall_number();
    let args: RawSyscallArgs = tf.get_syscall_args().try_into().unwrap();

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
            let result = sys_identify(IdentifyArgs::try_from(args).unwrap());
            log::debug!("identify syscall result is {:x?}", result);
            result.into()
        }

        AllocPage::SYSCALL_NO => {
            log::debug!(
                "handling alloc_page syscall with args {:?}",
                AllocPageArgs::try_from(args).unwrap()
            );
            let result = sys_alloc_page(AllocPageArgs::try_from(args).unwrap());
            log::debug!("alloc_page syscall result is {:?}", result);
            result.into()
        }

        no => {
            log::debug!(
                "received unknown syscall {} with args {:x?}",
                syscall_no,
                args
            );
            GenericReturn::UnsupportedSyscall.into()
        }
    };

    // write the result back to userspace
    let [a0, a1]: RawSyscallReturn = res.into();
    tf.write_syscall_result(a0, a1);

    // switch back to the calling frame
    tf
}
