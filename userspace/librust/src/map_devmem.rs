use syscall_abi::{
    map_devmem::{MapDevmem, MapDevmemArgs},
    CAddr, NoValue, SyscallResult,
};

use crate::syscalls::syscall;

pub fn map_devmem(devmem: CAddr, mem: CAddr, base: usize, len: usize) -> SyscallResult<NoValue> {
    syscall::<MapDevmem>(MapDevmemArgs {
        devmem,
        mem,
        base,
        len,
    })
}
