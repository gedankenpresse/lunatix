use crate::syscalls::syscall;
use syscall_abi::receive::{ReceiveArgs, ReceiveReturn};
use syscall_abi::send::NUM_DATA_REGS;
use syscall_abi::{CAddr, IpcTag, SyscallResult};

pub fn receive(cap: CAddr, label: usize, caps: &[CAddr]) -> SyscallResult<ReceiveReturn> {
    assert!(caps.len() <= NUM_DATA_REGS);
    let data_len = NUM_DATA_REGS - caps.len();

    syscall::<syscall_abi::receive::Receive>(ReceiveArgs {
        target: cap,
        tag: IpcTag::from_parts(label, caps.len() as u8, data_len as u8),
    })
}
