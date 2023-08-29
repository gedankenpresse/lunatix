use crate::caps::{Capability, PageIface, Tag};
use derivation_tree::tree::CursorRefMut;
use syscall_abi::alloc_page::{AllocPageArgs, AllocPageReturn};

pub(super) fn sys_alloc_page(
    task: &mut CursorRefMut<'_, '_, Capability>,
    args: AllocPageArgs,
) -> AllocPageReturn {
    let task = task.get_inner_task().unwrap();
    let mut cspace = task.get_cspace();
    let cspace = cspace.get_shared().unwrap();
    let cspace = cspace.get_inner_cspace().unwrap();

    let mem_cap = match unsafe { cspace.lookup_raw(args.src_mem) } {
        None => return AllocPageReturn::InvalidMemCAddr,
        Some(cap_ptr) => {
            // TODO Use a cursor to safely access the capability
            let cap = unsafe { &*cap_ptr };
            if *cap.get_tag() != Tag::Memory {
                return AllocPageReturn::InvalidMemCAddr;
            }
            cap
        }
    };

    let target_cap = match unsafe { cspace.lookup_raw(args.target_slot) } {
        None => return AllocPageReturn::InvalidTargetCAddr,
        Some(cap_ptr) => {
            let cap = unsafe { &mut *cap_ptr };
            if *cap.get_tag() != Tag::Uninit {
                return AllocPageReturn::InvalidTargetCAddr;
            }
            cap
        }
    };

    PageIface.derive(mem_cap, target_cap);
    AllocPageReturn::Success
}
