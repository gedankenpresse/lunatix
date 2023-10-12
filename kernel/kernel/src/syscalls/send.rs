use derivation_tree::tree::CursorRefMut;

use crate::{
    caps::{self, Capability, Error},
    SyscallContext,
};

use super::{
    asid_control::asid_control_send,
    devmem::devmem_send,
    irq::{irq_control_send, irq_send},
    mem::mem_send,
    page::page_send,
    task::task_send,
};

pub(super) fn sys_send(
    ctx: &mut SyscallContext,
    task: &mut CursorRefMut<'_, '_, Capability>,
    args: &[usize; 7],
) -> Result<(), caps::Error> {
    log::trace!("send args: {:?}", args);
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
    match cap.get_tag() {
        caps::Tag::Uninit => todo!("send for uninit unimplemented"),
        caps::Tag::Memory => mem_send(cspace, cap, &args[1..]),
        caps::Tag::CSpace => todo!("send for cspace unimplemented"),
        caps::Tag::VSpace => todo!("send for vspace unimplemented"),
        caps::Tag::Task => task_send(cspace, cap.get_inner_task().unwrap(), &args[1..]),
        caps::Tag::Page => page_send(cspace, cap.get_inner_page_mut().unwrap(), &args[1..]),
        caps::Tag::IrqControl => irq_control_send(ctx, cspace, cap, &args[1..]),
        caps::Tag::Irq => irq_send(ctx, cspace, cap.get_inner_irq().unwrap(), &args[1..]),
        caps::Tag::Notification => todo!("send for notification unimplemented"),
        caps::Tag::Devmem => devmem_send(cspace, cap.get_inner_devmem().unwrap(), &args[1..]),
        caps::Tag::AsidControl => {
            asid_control_send(cspace, cap.get_inner_asid_control().unwrap(), &args[1..])
        }
    }
}
