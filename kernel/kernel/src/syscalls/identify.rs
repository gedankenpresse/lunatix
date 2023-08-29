use crate::caps::{Capability, Tag};
use derivation_tree::tree::CursorRefMut;
use syscall_abi::identify::{CapabilityVariant, IdentifyArgs, IdentifyReturn};

pub(super) fn sys_identify(
    task: &mut CursorRefMut<'_, '_, Capability>,
    args: IdentifyArgs,
) -> IdentifyReturn {
    let task = task.get_inner_task().unwrap();
    let mut cspace = task.get_cspace();
    let cspace = cspace.get_shared().unwrap();
    let cspace = cspace.get_inner_cspace().unwrap();

    match unsafe { cspace.lookup_raw(args.caddr) } {
        None => IdentifyReturn::InvalidCAddr,
        Some(cap_ptr) => {
            // TODO Use a cursor to safely access the capability
            let cap = unsafe { &*cap_ptr };
            IdentifyReturn::Success(match cap.get_tag() {
                Tag::Uninit => CapabilityVariant::Uninit,
                Tag::Memory => CapabilityVariant::Memory,
                Tag::CSpace => CapabilityVariant::CSpace,
                Tag::VSpace => CapabilityVariant::VSpace,
                Tag::Task => CapabilityVariant::Task,
                Tag::Page => CapabilityVariant::Page,
            })
        }
    }
}
