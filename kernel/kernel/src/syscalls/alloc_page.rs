use crate::caps::{PageIface, Tag};
use crate::sched;
use syscall_abi::alloc_page::{AllocPageArgs, AllocPageReturn};

pub(super) fn sys_alloc_page(args: AllocPageArgs) -> AllocPageReturn {
    let cspace = sched::cspace().get_cspace().unwrap();
    let cspace = cspace.as_ref();

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
