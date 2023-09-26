use crate::syscalls::syscall;
use syscall_abi::task_assign_cspace::{
    TaskAssignCSpace, TaskAssignCSpaceArgs, TaskAssignCSpaceReturn,
};
use syscall_abi::CAddr;

pub fn task_assign_cspace(cspace_addr: CAddr, task_addr: CAddr) -> TaskAssignCSpaceReturn {
    syscall::<TaskAssignCSpace>(TaskAssignCSpaceArgs {
        cspace_addr,
        task_addr,
    })
    .unwrap()
}
