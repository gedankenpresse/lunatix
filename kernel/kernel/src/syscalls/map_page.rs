use crate::caps::Tag;
use crate::sched;
use bitflags::Flags;
use core::mem;
use riscv::pt::{EntryFlags, MemoryPage};
use syscall_abi::map_page::{MapPageArgs, MapPageReturn};

pub(super) fn sys_map_page(args: MapPageArgs) -> MapPageReturn {
    let cspace = sched::cspace().get_cspace().unwrap();
    let cspace = cspace.as_ref();

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

    // TODO Replace constant pointer with something else.
    // either use some sort of allocator to automatically assign new VAddrs or have the user pass it in the syscall
    const VADDR: usize = 0x42_000;
    if let Err(NoMem) = vspace_cap.get_inner_vspace().unwrap().map_range(
        mem_cap,
        VADDR,
        mem::size_of::<MemoryPage>(),
        (EntryFlags::UserReadable | EntryFlags::Read | EntryFlags::Write).bits() as usize,
    ) {
        return MapPageReturn::NoMem;
    }

    MapPageReturn::Success(VADDR as *mut u8)
}