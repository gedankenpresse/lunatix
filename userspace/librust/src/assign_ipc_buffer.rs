use crate::syscalls::syscall;
use syscall_abi::assign_ipc_buffer::{AssignIpcBuffer, AssignIpcBufferArgs, AssignIpcBufferReturn};
use syscall_abi::CAddr;

pub fn assign_ipc_buffer(page: CAddr) -> AssignIpcBufferReturn {
    syscall::<AssignIpcBuffer>(AssignIpcBufferArgs { page }).unwrap()
}
