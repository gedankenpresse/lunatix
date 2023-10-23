use derivation_tree::tree::CursorRefMut;
use libkernel::mem::ptrs::MappedConstPtr;
use syscall_abi::MapFlags;

use crate::{
    caps::{page::map_page, CSpace, Capability, Error, Page, Tag},
    syscalls::utils,
};

pub fn page_send(cspace: &CSpace, page: &mut Page, args: &[usize]) -> Result<(), Error> {
    const MAP: usize = 0;
    const UNMAP: usize = 1;
    const PADDR: usize = 2;

    match args[0] {
        MAP => {
            let [mem, vspace, addr, flags, _] = args[1..] else {
                panic!("not enough arguments")
            };
            let mem_cap = unsafe { utils::lookup_cap(cspace, mem, Tag::Memory) }?;
            let vspace_cap = unsafe { utils::lookup_cap(cspace, vspace, Tag::VSpace) }?;
            let flags = MapFlags::from_bits(flags).ok_or(Error::InvalidArg)?;
            map_page(
                page,
                mem_cap.get_inner_memory().unwrap(),
                vspace_cap.get_inner_vspace().unwrap(),
                flags,
                addr,
            )
        }
        UNMAP => {
            page.unmap();
            Ok(())
        }
        _ => Err(Error::Unsupported),
    }
}

pub(crate) fn page_paddr(
    ctx: &mut crate::SyscallContext,
    task: &mut CursorRefMut<'_, '_, Capability>,
    args: &[usize; 7],
) -> Result<usize, Error> {
    let task = task.get_inner_task().unwrap();
    let mut cspace = task.get_cspace();
    let cspace = cspace.get_shared().unwrap();
    let cspace = cspace.get_inner_cspace().unwrap();

    let cap = unsafe {
        cspace
            .lookup_raw(args[0])
            .ok_or(Error::InvalidCAddr)?
            .as_mut()
            .unwrap()
    };
    let page = cap.get_inner_page().map_err(|_| Error::InvalidCap)?;
    let phys = MappedConstPtr::from(page.kernel_addr as *const _)
        .as_direct()
        .raw();
    Ok(phys as usize)
}
