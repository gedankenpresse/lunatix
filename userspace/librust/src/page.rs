use crate::syscalls::send;
use syscall_abi::{CAddr, FromRawSysResponse, MapFlags, NoValue, SyscallResult};

pub fn map_page(
    page: CAddr,
    vspace: CAddr,
    mem: CAddr,
    addr: usize,
    flags: MapFlags,
) -> SyscallResult<NoValue> {
    const MAP: usize = 0;
    SyscallResult::from_response(send(page, MAP, mem, vspace, addr, flags.bits(), 0))
}

pub fn unmap_page(page: CAddr) -> SyscallResult<NoValue> {
    const UNMAP: usize = 1;
    let res = send(page, UNMAP, 0, 0, 0, 0, 0);
    SyscallResult::from_response(res)
}
