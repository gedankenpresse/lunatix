use crate::caps::{Capability, Tag};
use crate::syscalls::ipc::page::page_call;
use crate::KernelContext;
use derivation_tree::tree::CursorRefMut;
use syscall_abi::call::Call;
use syscall_abi::{SyscallBinding, SyscallError};

pub(super) fn sys_call(
    _ctx: &mut KernelContext,
    task: &mut CursorRefMut<'_, '_, Capability>,
    args: <Call as SyscallBinding>::CallArgs,
) -> <Call as SyscallBinding>::Return {
    let task = task.get_inner_task().unwrap();
    let mut cspace = task.get_cspace();
    let cspace = cspace.get_shared().unwrap();
    let cspace = cspace.get_inner_cspace().unwrap();

    let cap = unsafe {
        cspace
            .resolve_caddr(args.target)
            .ok_or(SyscallError::InvalidCAddr)?
            .as_mut()
            .unwrap()
    };
    log::debug!("dispatching call to {:?} capability", cap.get_tag());
    match cap.get_tag() {
        Tag::Uninit => todo!("call for uninit unimplemented"),
        Tag::Memory => todo!("call for memory unimplemented"),
        Tag::CSpace => todo!("call for cspace unimplemented"),
        Tag::VSpace => todo!("call for vspace unimplemented"),
        Tag::Task => todo!("call to task unimplemented"),
        Tag::Page => page_call(cspace, cap.get_inner_page_mut().unwrap(), args),
        Tag::IrqControl => todo!("call for irq-control unimplemented"),
        Tag::Irq => todo!("call for irq unimplemented"),
        Tag::Notification => todo!("call for notification unimplemented"),
        Tag::Devmem => todo!("call for devmem unimplemented"),
        Tag::AsidControl => todo!("call for asid-control unimplemented"),
    }
}
