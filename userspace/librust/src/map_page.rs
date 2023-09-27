use crate::syscalls::syscall;
use syscall_abi::map_page::{MapPage, MapPageArgs, MapPageFlag};
use syscall_abi::{CAddr, NoValue, SyscallResult};

pub fn map_page(
    page: CAddr,
    vspace: CAddr,
    mem: CAddr,
    addr: usize,
    flags: MapPageFlag,
) -> SyscallResult<NoValue> {
    syscall::<MapPage>(MapPageArgs {
        page,
        vspace,
        mem,
        addr,
        flags,
    })
}
