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
