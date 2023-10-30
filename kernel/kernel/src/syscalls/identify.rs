use crate::caps::{Capability, Tag};
use derivation_tree::tree::CursorRefMut;
use syscall_abi::{
    identify::{CapabilityVariant, IdentifyArgs},
    Error, SyscallResult,
};

pub(super) fn sys_identify(
    task: &mut CursorRefMut<'_, '_, Capability>,
    args: IdentifyArgs,
) -> SyscallResult<CapabilityVariant> {
    let task = task.get_inner_task().unwrap();
    let mut cspace = task.get_cspace();
    let cspace = cspace.get_shared().unwrap();
    let cspace = cspace.get_inner_cspace().unwrap();

    let cap_ptr = unsafe { cspace.lookup_raw(args.caddr) }
        .ok_or(Error::InvalidCAddr)?
        .0;
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
