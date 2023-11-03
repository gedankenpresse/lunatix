use crate::syscalls::{call, send};
use syscall_abi::{CAddr, MapFlags, NoValue, SyscallResult};

pub fn map_page(
    page: CAddr,
    vspace: CAddr,
    mem: CAddr,
    addr: usize,
    flags: MapFlags,
) -> SyscallResult<NoValue> {
    const MAP: usize = 0;
    send(page, MAP, &[mem, vspace], &[addr, flags.bits()])
}

pub fn unmap_page(page: CAddr) -> SyscallResult<NoValue> {
    const UNMAP: usize = 1;
    send(page, UNMAP, &[], &[])
}

pub fn get_paddr(page: CAddr) -> SyscallResult<usize> {
    const GET_PADDR: usize = 2;
    call(page, GET_PADDR, &[], &[]).map(|data| data[0])
}
