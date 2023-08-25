use crate::syscalls::syscall;
use syscall_abi::alloc_page::{AllocPage, AllocPageArgs, AllocPageReturn};
use syscall_abi::CAddr;

pub fn alloc_page(src_mem: CAddr, target_slot: CAddr) -> AllocPageReturn {
    syscall::<AllocPage>(AllocPageArgs {
        src_mem,
        target_slot,
    })
    .unwrap()
}
