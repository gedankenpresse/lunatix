use crate::syscalls::syscall;
use syscall_abi::task_assign_control_registers::{
    TaskAssignControlRegisters, TaskAssignControlRegistersArgs, TaskAssignControlRegistersReturn,
};
use syscall_abi::CAddr;

pub fn task_assign_control_registers(
    task_addr: CAddr,
    pc: usize,
    sp: usize,
    fp: usize,
    gp: usize,
) -> TaskAssignControlRegistersReturn {
    syscall::<TaskAssignControlRegisters>(TaskAssignControlRegistersArgs {
        task_addr,
        pc,
        sp,
        fp,
        gp,
    })
    .unwrap()
}
