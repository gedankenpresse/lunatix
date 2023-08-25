use crate::caps::Tag;
use crate::sched;
use syscall_abi::identify::{CapabilityVariant, IdentifyArgs, IdentifyReturn};

pub(super) fn sys_identify(args: IdentifyArgs) -> IdentifyReturn {
    let cspace = sched::cspace().get_cspace().unwrap();
    let cspace = cspace.as_ref();

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
