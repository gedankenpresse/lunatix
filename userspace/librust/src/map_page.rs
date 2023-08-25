use crate::syscalls::syscall;
use syscall_abi::map_page::{MapPage, MapPageArgs, MapPageReturn};
use syscall_abi::CAddr;

pub fn map_page(page: CAddr, vspace: CAddr, mem: CAddr) -> MapPageReturn {
    syscall::<MapPage>(MapPageArgs { page, vspace, mem }).unwrap()
}
