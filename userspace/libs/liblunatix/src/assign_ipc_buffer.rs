use crate::syscalls::syscall;
use syscall_abi::assign_ipc_buffer::{AssignIpcBuffer, AssignIpcBufferArgs};
use syscall_abi::{CAddr, NoValue, SyscallResult};

pub fn assign_ipc_buffer(page: CAddr) -> SyscallResult<NoValue> {
    syscall::<AssignIpcBuffer>(AssignIpcBufferArgs { page })
}
