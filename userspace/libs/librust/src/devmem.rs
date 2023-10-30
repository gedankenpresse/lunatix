use syscall_abi::{CAddr, FromRawSysResponse, NoValue, SyscallResult};

use crate::syscalls::send;

pub fn devmem_map(
    devmem: CAddr,
    mem: CAddr,
    vspace: CAddr,
    base: usize,
    len: usize,
) -> SyscallResult<NoValue> {
    const MAP: usize = 1;
    send(devmem, MAP, &[mem, vspace], &[base, len, 0])
}
