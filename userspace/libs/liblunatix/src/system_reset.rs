use crate::syscalls::syscall;
use syscall_abi::system_reset::{ResetReason, ResetType, SystemReset, SystemResetArgs};

pub fn system_reset(typ: ResetType, reason: ResetReason) -> ! {
    syscall::<SystemReset>(SystemResetArgs { typ, reason }).unwrap();
    unreachable!()
}
