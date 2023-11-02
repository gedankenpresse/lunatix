use crate::caps::{Capability, Tag};
use derivation_tree::tree::CursorRefMut;
use syscall_abi::identify::Identify;
use syscall_abi::{
    identify::{CapabilityVariant, IdentifyArgs},
    SyscallBinding, SyscallError, SyscallResult,
};

pub(super) fn sys_identify(
    task: &mut CursorRefMut<'_, '_, Capability>,
    args: <Identify as SyscallBinding>::CallArgs,
) -> <Identify as SyscallBinding>::Return {
    let task = task.get_inner_task().unwrap();
    let mut cspace = task.get_cspace();
    let cspace = cspace.get_shared().unwrap();
    let cspace = cspace.get_inner_cspace().unwrap();

    let cap_ptr = unsafe { cspace.resolve_caddr(args.caddr) }.ok_or(SyscallError::InvalidCAddr)?;

    // TODO Use a cursor to safely access the capability
    let cap = unsafe { &*cap_ptr };
    let tag = cap.get_tag();
    let variant = match tag {
        Tag::Uninit => CapabilityVariant::Uninit,
        Tag::Memory => CapabilityVariant::Memory,
        Tag::CSpace => CapabilityVariant::CSpace,
        Tag::VSpace => CapabilityVariant::VSpace,
        Tag::Task => CapabilityVariant::Task,
        Tag::Page => CapabilityVariant::Page,
        Tag::IrqControl => CapabilityVariant::IrqControl,
        Tag::Irq => CapabilityVariant::Irq,
        Tag::Notification => CapabilityVariant::Notification,
        Tag::Devmem => CapabilityVariant::Devmem,
        Tag::AsidControl => CapabilityVariant::AsidControl,
    };
    Ok(variant)
}
