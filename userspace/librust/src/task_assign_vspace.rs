use crate::syscalls::syscall;
use syscall_abi::task_assign_vspace::{TaskAssignVSpace, TaskAssignVSpaceArgs};
use syscall_abi::{CAddr, NoValue, SyscallResult};

pub fn task_assign_vspace(vspace_addr: CAddr, task_addr: CAddr) -> SyscallResult<NoValue> {
    syscall::<TaskAssignVSpace>(TaskAssignVSpaceArgs {
        vspace_addr,
        task_addr,
    })
}
