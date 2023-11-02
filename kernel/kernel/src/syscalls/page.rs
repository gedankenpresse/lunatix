use derivation_tree::tree::CursorRefMut;
use riscv::mem::ptrs::MappedConstPtr;
use syscall_abi::send::SendArgs;
use syscall_abi::{MapFlags, RawSyscallArgs, SyscallReturnData};

use crate::{
    caps::{page::map_page, CSpace, Capability, Page, SyscallError, Tag},
    syscalls::utils,
};

pub fn page_send(cspace: &CSpace, page: &mut Page, args: &SendArgs) -> Result<(), SyscallError> {
    const MAP: usize = 0;
    const UNMAP: usize = 1;
    const PADDR: usize = 2;

    match args.label() {
        MAP => {
            let [mem, vspace] = args.cap_args() else {
                panic!("not enough cap arguments")
            };

            let [addr, flags] = args.data_args() else {
                panic!("not enough data arguments")
            };
            let mem_cap = unsafe { utils::lookup_cap(cspace, *mem, Tag::Memory) }?;
            let vspace_cap = unsafe { utils::lookup_cap(cspace, *vspace, Tag::VSpace) }?;
            let flags = MapFlags::from_bits(*flags).ok_or(SyscallError::InvalidArg)?;
            map_page(
                page,
                mem_cap.get_inner_memory().unwrap(),
                vspace_cap.get_inner_vspace().unwrap(),
                flags,
                *addr,
            )
        }
        UNMAP => {
            page.unmap();
            Ok(())
        }
        _ => Err(SyscallError::Unsupported),
    }
}

pub(crate) fn page_paddr(
    _ctx: &mut crate::SyscallContext,
    task: &mut CursorRefMut<'_, '_, Capability>,
    args: &RawSyscallArgs,
) -> Result<SyscallReturnData, SyscallError> {
    let task = task.get_inner_task().unwrap();
    let mut cspace = task.get_cspace();
    let cspace = cspace.get_shared().unwrap();
    let cspace = cspace.get_inner_cspace().unwrap();

    let cap = unsafe {
        cspace
            .resolve_caddr(args[0].into())
            .ok_or(SyscallError::InvalidCAddr)?
            .as_mut()
            .unwrap()
    };
    let page = cap.get_inner_page().map_err(|_| SyscallError::InvalidCap)?;
    let phys = MappedConstPtr::from(page.kernel_addr as *const _)
        .as_direct()
        .raw();
    Ok([phys as usize, 0, 0, 0, 0, 0, 0])
}
