use crate::syscalls::syscall;
use syscall_abi::map_page::{MapPage, MapPageArgs, MapPageFlag, MapPageReturn};
use syscall_abi::CAddr;

pub fn map_page(
    page: CAddr,
    vspace: CAddr,
    mem: CAddr,
    addr: usize,
    flags: MapPageFlag,
) -> MapPageReturn {
    syscall::<MapPage>(MapPageArgs {
        page,
        vspace,
        mem,
        addr,
        flags,
    })
    .unwrap()
}
