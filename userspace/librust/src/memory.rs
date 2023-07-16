use crate::Variant;
use crate::syscalls;
use crate::ipc;
use syscalls::syscall;

const ALLOC: usize = 0;


pub fn allocate(mem: usize, target: usize, variant: Variant, size: usize) -> Result<usize, crate::Error> {
    const LABEL: usize = ALLOC;
    const NCAP: u8 = 1;
    let nparam: u8 = if size == 0 { 1 } else { 2 };
    let tag: ipc::Tag = ipc::Tag::from_parts(LABEL, NCAP, nparam);
    crate::println!("alloc/tag: {tag:?} label: {} caps: {} params: {}", tag.label(), tag.ncaps(), tag.nparams());
    return syscall(syscalls::SYS_SEND, mem, tag.0, target, variant as usize, size, 0, 0);
}