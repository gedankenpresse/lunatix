use crate::syscalls::syscall;
use syscall_abi::task_assign_cspace::{TaskAssignCSpace, TaskAssignCSpaceArgs};
use syscall_abi::{CAddr, NoValue, SyscallResult};

pub fn task_assign_cspace(cspace_addr: CAddr, task_addr: CAddr) -> SyscallResult<NoValue> {
    syscall::<TaskAssignCSpace>(TaskAssignCSpaceArgs {
        cspace_addr,
        task_addr,
    })
}
