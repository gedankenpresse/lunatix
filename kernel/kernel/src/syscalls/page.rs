use syscall_abi::MapFlags;

use crate::{
    caps::{page::map_page, CSpace, Error, Page, Tag},
    syscalls::utils,
};

pub fn page_send(cspace: &CSpace, page: &mut Page, args: &[usize]) -> Result<(), Error> {
    const MAP: usize = 0;

    match args[0] {
        MAP => {
            let [mem, vspace, addr, flags, _] = args[1..] else { panic!("not enough arguments")};
            let mem_cap = unsafe { utils::lookup_cap(cspace, mem, Tag::Memory) }?;
            let vspace_cap = unsafe { utils::lookup_cap(cspace, vspace, Tag::VSpace) }?;
            let flags = MapFlags::from_bits(flags).ok_or(Error::InvalidArg)?;
            map_page(
                page,
                mem_cap.get_inner_memory().unwrap(),
                vspace_cap.get_inner_vspace().unwrap(),
                flags,
                addr,
            )
        }
        _ => Err(Error::Unsupported),
    }
}
