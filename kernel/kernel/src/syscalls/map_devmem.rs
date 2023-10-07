use core::arch::asm;

use derivation_tree::tree::CursorRefMut;
use libkernel::mem::PAGESIZE;
use riscv::pt::EntryFlags;
use syscall_abi::map_devmem::MapDevmem as Current;
use syscall_abi::{map_devmem::MapDevmemArgs, SyscallBinding};
use syscall_abi::{NoValue, SysError};

use crate::caps::{Capability, Tag};

pub(super) fn sys_map_devmem(
    task: &mut CursorRefMut<'_, '_, Capability>,
    args: <Current as SyscallBinding>::CallArgs,
) -> <Current as SyscallBinding>::Return {
    let task = task.get_inner_task().unwrap();
    let mut cspace = task.get_cspace();
    let cspace = cspace.get_shared().unwrap();
    let cspace = cspace.get_inner_cspace().unwrap();

    let mut vspace = task.get_vspace();
    let vspace = vspace.get_shared().unwrap();
    let vspace = vspace.get_inner_vspace().unwrap();

    let cap_ptr = unsafe { cspace.lookup_raw(args.devmem) }.ok_or(SysError::InvalidCaddr)?;
    let devmem = unsafe { cap_ptr.as_mut().unwrap() };
    let devmem = devmem.get_inner_devmem().unwrap();
    let Some(entry) = devmem.inner_state.iter().find(|&entry| {
        let r = entry.borrow();
        let Some(e) = *r else { return false };
        return e.base == args.base && e.len == args.len;
    }) else { return Err(SysError::NotFound) };
    let entry = entry.borrow().unwrap();

    let mem = unsafe { super::utils::lookup_cap(cspace, args.mem, Tag::Memory).unwrap() };
    for offset in (0..args.len).step_by(PAGESIZE) {
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
    Ok(NoValue)
}
