use crate::syscalls::syscall;
use syscall_abi::task_assign_vspace::{
    TaskAssignVSpace, TaskAssignVSpaceArgs, TaskAssignVSpaceReturn,
};
use syscall_abi::CAddr;

pub fn task_assign_vspace(vspace_addr: CAddr, task_addr: CAddr) -> TaskAssignVSpaceReturn {
    syscall::<TaskAssignVSpace>(TaskAssignVSpaceArgs {
        vspace_addr,
        task_addr,
    })
    .unwrap()
}
