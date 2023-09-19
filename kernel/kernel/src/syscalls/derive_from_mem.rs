use derivation_tree::tree::CursorRefMut;
use syscall_abi::derive_from_mem::{DeriveFromMemArgs, DeriveFromMemReturn};
use syscall_abi::identify::CapabilityVariant;
use crate::caps::{Capability, CSpaceIface, MemoryIface, PageIface, Tag, TaskIface, VSpaceIface};

pub(super) fn sys_derive_from_mem(task: &mut CursorRefMut<'_, '_, Capability>, args: DeriveFromMemArgs) -> DeriveFromMemReturn {
    // get basic caps from task
    let task = task.get_inner_task().unwrap();
    let mut cspace = task.get_cspace();
    let cspace = cspace.get_shared().unwrap();
    let cspace = cspace.get_inner_cspace().unwrap();

    // get valid memory cap from task
    let mem_cap = match unsafe { cspace.lookup_raw(args.src_mem) } {
        None => return DeriveFromMemReturn::InvalidMemCAddr,
        Some(cap_ptr) => {
            // TODO Use a cursor to safely access the capability
            let cap = unsafe { &*cap_ptr };
            if *cap.get_tag() != Tag::Memory {
                return DeriveFromMemReturn::InvalidMemCAddr;
            }
            cap
        }
    };

    // get valid uninitialized target cap from task
    let target_cap = match unsafe { cspace.lookup_raw(args.target_slot) } {
        None => return DeriveFromMemReturn::InvalidTargetCAddr,
        Some(cap_ptr) => {
            let cap = unsafe { &mut *cap_ptr };
            if *cap.get_tag() != Tag::Uninit {
                return DeriveFromMemReturn::InvalidTargetCAddr;
            }
            cap
        }
    };

    // derive the correct capability
    match args.target_cap {
        CapabilityVariant::Uninit => DeriveFromMemReturn::CannotBeDerived,
        CapabilityVariant::Memory => unimplemented!("memory cannot yet be derived"),
        CapabilityVariant::CSpace => {
            CSpaceIface.derive(mem_cap, target_cap, args.size.unwrap());
            DeriveFromMemReturn::Success
        },
        CapabilityVariant::VSpace => {
            VSpaceIface.derive(mem_cap, target_cap);
            DeriveFromMemReturn::Success
        }
        CapabilityVariant::Task => {
            TaskIface.derive(mem_cap, target_cap);
            DeriveFromMemReturn::Success
        }
        CapabilityVariant::Page => {
            PageIface.derive(mem_cap, target_cap);
            DeriveFromMemReturn::Success
        }
    }
}
