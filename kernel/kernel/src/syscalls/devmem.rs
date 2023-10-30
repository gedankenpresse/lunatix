use core::arch::asm;
use riscv::pt::{EntryFlags, PAGESIZE};
use syscall_abi::send::SendArgs;
use syscall_abi::CAddr;

use crate::{
    caps::{CSpace, Devmem, Error, Tag},
    syscalls::utils,
};

pub fn devmem_send(cspace: &CSpace, devmem: &Devmem, args: &SendArgs) -> Result<(), Error> {
    const MAP: usize = 1;
    match args.label() {
        MAP => devmem_map(
            cspace,
            devmem,
            args.cap_args()[0],
            args.cap_args()[1],
            args.data_args()[0],
            args.data_args()[1],
        ),
        _ => Err(Error::Unsupported),
    }
}

fn devmem_map(
    cspace: &CSpace,
    devmem: &Devmem,
    mem_addr: CAddr,
    vspace_addr: CAddr,
    base: usize,
    len: usize,
) -> Result<(), Error> {
    let mem = unsafe { utils::lookup_cap(cspace, mem_addr, Tag::Memory)? };
    let vspace = unsafe { utils::lookup_cap_mut(cspace, vspace_addr, Tag::VSpace)? };
    let vspace = vspace.get_inner_vspace_mut().unwrap();

    let Some(entry) = devmem.inner_state.iter().find(|&entry| {
        let r = entry.borrow();
        let Some(e) = *r else { return false };
        return e.base == base && e.len == len;
    }) else {
        return Err(Error::InvalidArg);
    };
    let entry = entry.borrow().unwrap();

    for offset in (0..len).step_by(PAGESIZE) {
        vspace
            .map_address(
                mem.get_inner_memory().unwrap(),
                entry.base + offset,
                entry.base + offset,
                EntryFlags::Read | EntryFlags::Write | EntryFlags::UserReadable,
            )
            .unwrap();
    }
    unsafe {
        asm!("sfence.vma");
    }
    Ok(())
}
