use crate::caps::{Capability, Tag};
use core::arch::asm;
use core::mem;
use derivation_tree::tree::CursorRefMut;
use riscv::pt::{EntryFlags, MemoryPage};
use syscall_abi::map_page::{MapPageArgs, MapPageReturn};

pub(super) fn sys_map_page(
    task: &mut CursorRefMut<'_, '_, Capability>,
    args: MapPageArgs,
) -> MapPageReturn {
    let task = task.get_inner_task().unwrap();
    let mut cspace = task.get_cspace();
    let cspace = cspace.get_shared().unwrap();
    let cspace = cspace.get_inner_cspace().unwrap();

    let page_cap = match unsafe { cspace.lookup_raw(args.page) } {
        None => return MapPageReturn::InvalidPageCAddr,
        Some(cap_ptr) => {
            // TODO Use a cursor to safely access the capability
            let cap = unsafe { &*cap_ptr };
            if *cap.get_tag() != Tag::Page {
                return MapPageReturn::InvalidPageCAddr;
            }
            cap
        }
    };

    let vspace_cap = match unsafe { cspace.lookup_raw(args.vspace) } {
        None => return MapPageReturn::InvalidVSpaceCAddr,
        Some(cap_ptr) => {
            let cap = unsafe { &*cap_ptr };
            if *cap.get_tag() != Tag::VSpace {
                return MapPageReturn::InvalidVSpaceCAddr;
            }
            cap
        }
    };

    let mem_cap = match unsafe { cspace.lookup_raw(args.mem) } {
        None => return MapPageReturn::InvalidMemCAddr,
        Some(cap_ptr) => {
            let cap = unsafe { &*cap_ptr };
            if *cap.get_tag() != Tag::Memory {
                return MapPageReturn::InvalidMemCAddr;
            }
            cap
        }
    };

    // either use some sort of allocator to automatically assign new VAddrs or have the user pass it in the syscall
    match vspace_cap.get_inner_vspace().unwrap().map_range(
        mem_cap,
        args.addr,
        mem::size_of::<MemoryPage>(),
        (EntryFlags::UserReadable | EntryFlags::Read | EntryFlags::Write).bits() as usize,
    ) {
        Err(_) => MapPageReturn::NoMem,
        Ok(_) => {
            unsafe { asm!("sfence.vma") };
            MapPageReturn::Success
        }
    }
}
