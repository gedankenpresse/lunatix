use syscall_abi::{CAddr, NoValue, SyscallResult};

use crate::syscalls::send;

pub fn asid_assign(asid_ctrl: CAddr, vspace: CAddr) -> SyscallResult<NoValue> {
    const ASSIGN: usize = 1234;
    send(asid_ctrl, ASSIGN, &[vspace], &[])
}
