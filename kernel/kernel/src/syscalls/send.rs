use derivation_tree::tree::CursorRefMut;
use syscall_abi::send::Send;
use syscall_abi::{NoValue, SyscallBinding};

use crate::{
    caps::{self, Capability, SyscallError},
    KernelContext,
};

use super::ipc;

pub(super) fn sys_send(
    ctx: &mut KernelContext,
    task: &mut CursorRefMut<'_, '_, Capability>,
    args: <Send as SyscallBinding>::CallArgs,
) -> <Send as SyscallBinding>::Return {
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
    log::debug!("dispatching send to {:?} capability", cap.get_tag());
    match cap.get_tag() {
        caps::Tag::Uninit => todo!("send for uninit unimplemented"),
        caps::Tag::Memory => ipc::mem::mem_send(cspace, cap, &args),
        caps::Tag::CSpace => todo!("send for cspace unimplemented"),
        caps::Tag::VSpace => todo!("send for vspace unimplemented"),
        caps::Tag::Task => ipc::task::task_send(cspace, cap.get_inner_task().unwrap(), &args),
        caps::Tag::Page => ipc::page::page_send(cspace, cap.get_inner_page_mut().unwrap(), &args),
        caps::Tag::IrqControl => ipc::irq::irq_control_send(ctx, cspace, cap, &args),
        caps::Tag::Irq => ipc::irq::irq_send(ctx, cspace, cap.get_inner_irq().unwrap(), &args),
        caps::Tag::Notification => todo!("send for notification unimplemented"),
        caps::Tag::Devmem => {
            ipc::devmem::devmem_send(cspace, cap.get_inner_devmem().unwrap(), &args)
        }
        caps::Tag::AsidControl => ipc::asid_control::asid_control_send(
            cspace,
            cap.get_inner_asid_control().unwrap(),
            &args,
        ),
    }?;
    Ok(NoValue)
}
