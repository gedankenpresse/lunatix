use crate::syscalls::send;
use syscall_abi::{CAddr, FromRawSysResponse, NoValue, SyscallResult};

pub fn task_assign_cspace(cspace: CAddr, task: CAddr) -> SyscallResult<NoValue> {
    const ASSIGN_CSPACE: usize = 3;
    let res = send(task, ASSIGN_CSPACE, cspace, 0, 0, 0, 0);
    SyscallResult::from_response(res)
}

pub fn task_assign_vspace(vspace: CAddr, task: CAddr) -> SyscallResult<NoValue> {
    const ASSIGN_VSPACE: usize = 2;
    let res = send(task, ASSIGN_VSPACE, vspace, 0, 0, 0, 0);
    SyscallResult::from_response(res)
}

pub fn task_assign_control_registers(
    task: CAddr,
    pc: usize,
    sp: usize,
    fp: usize,
    gp: usize,
) -> SyscallResult<NoValue> {
    const ASSIGN_REGS: usize = 1;
    let res = send(task, ASSIGN_REGS, pc, sp, fp, gp, 0);
    SyscallResult::from_response(res)
}
