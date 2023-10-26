use crate::syscalls::send;
use syscall_abi::{CAddr, FromRawSysResponse, NoValue, SyscallResult};

pub fn task_assign_cspace(cspace: CAddr, task: CAddr) -> SyscallResult<NoValue> {
    const ASSIGN_CSPACE: u16 = 3;
    send(task, ASSIGN_CSPACE, &[cspace], &[0, 0, 0, 0])
}

pub fn task_assign_vspace(vspace: CAddr, task: CAddr) -> SyscallResult<NoValue> {
    const ASSIGN_VSPACE: u16 = 2;
    send(task, ASSIGN_VSPACE, &[vspace], &[0, 0, 0, 0])
}

pub fn task_assign_control_registers(
    task: CAddr,
    pc: usize,
    sp: usize,
    fp: usize,
    gp: usize,
) -> SyscallResult<NoValue> {
    const ASSIGN_REGS: u16 = 1;
    send(task, ASSIGN_REGS, &[], &[pc, sp, fp, gp, 0])
}
