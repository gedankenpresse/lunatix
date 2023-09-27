use crate::caps::{Capability, Tag};
use crate::syscalls::utils;
use crate::virtmem::KernelMapper;
use core::arch::asm;
use derivation_tree::tree::CursorRefMut;
use libkernel::mem::PAGESIZE;
use riscv::pt::EntryFlags;
use riscv::PhysMapper;
use syscall_abi::map_page::{MapPageArgs, MapPageFlag};
use syscall_abi::{NoValue, SysError, SyscallResult};

pub(super) fn sys_map_page(
    task: &mut CursorRefMut<'_, '_, Capability>,
    args: MapPageArgs,
) -> SyscallResult<NoValue> {
    let task = task.get_inner_task().unwrap();
    let mut cspace = task.get_cspace();
    let cspace = cspace.get_shared().unwrap();
    let cspace = cspace.get_inner_cspace().unwrap();

    let page_cap = unsafe { utils::lookup_cap(cspace, args.page, Tag::Page) }?;

    let vspace_cap = unsafe { utils::lookup_cap(cspace, args.vspace, Tag::VSpace) }?;

    let mem_cap = unsafe { utils::lookup_cap(cspace, args.mem, Tag::Memory) }?;

    // compute flags with which to map from arguments
    let mut flags = EntryFlags::UserReadable;
    if args.flags.contains(MapPageFlag::READ) {
        flags |= EntryFlags::Read;
    }
    if args.flags.contains(MapPageFlag::WRITE) {
        flags |= EntryFlags::Write;
    }
    if args.flags.contains(MapPageFlag::EXEC) {
        flags |= EntryFlags::Execute
    }

    // map the page
    assert_eq!(
        args.addr & !(PAGESIZE - 1),
        args.addr,
        "page address is not page-aligned"
    );
    match vspace_cap.get_inner_vspace().unwrap().map_address(
        mem_cap.get_inner_memory().unwrap(),
        args.addr,
        unsafe { KernelMapper.mapped_to_phys(page_cap.get_inner_page().unwrap().kernel_addr) }
            as usize,
        flags,
    ) {
        Err(_) => Err(SysError::NoMem),
        Ok(_) => {
            unsafe { asm!("sfence.vma") };
            Ok(NoValue)
        }
    }
}
