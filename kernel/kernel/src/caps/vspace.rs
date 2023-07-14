use core::mem::MaybeUninit;

use crate::caps;
use caps::errors::NoMem;
use libkernel::mem::{EntryFlags, MemoryPage, PageTable};

use crate::virtmem;

pub struct VSpace {
    pub(crate) root: *mut PageTable,
}

impl VSpace {
    pub(crate) fn init(slot: &mut caps::CSlot, mem: &mut caps::CNode) -> Result<(), caps::Error> {
        let memref  = mem.get_memory_mut().unwrap();
        log::debug!("alloc");
        let root = memref.elem.alloc_pages_raw(1)?;
        log::debug!("init copy");
        let root = PageTable::init_copy(root.cast::<MaybeUninit<MemoryPage>>(), unsafe {
            crate::KERNEL_ROOT_PT
                .as_mapped()
                .raw()
                .as_ref()
                .expect("No Kernel Root Page Table found")
        });

        slot.set(Self { root }).unwrap();
        unsafe { mem.link_derive(slot.cap.as_link())};
        Ok(())
    }

    // Allocate a range of virtual addresses
    // Creates needed pages and page tables from given memory
    pub(crate) fn map_range(
        &self,
        mem: &mut caps::CNode,
        vaddr_base: usize,
        size: usize,
        flags: usize,
    ) -> Result<(), NoMem> {
        let memref = mem.get_memory_mut().unwrap();
        log::debug!("map range, root: {:p}", self.root);
        virtmem::map_range_alloc(
            memref.elem.get_inner_mut(),
            unsafe { self.root.as_mut().unwrap() },
            vaddr_base,
            size,
            EntryFlags::from_bits_truncate(flags as u64),
        );
        Ok(())
    }

    // Allocate a page
    // Currently implicitly allcates a new page from given memory, but should in theory be provided page
    pub(crate) fn map_page(
        &self,
        mem: &mut caps::Memory,
        vaddr: usize,
        flags: usize,
    ) -> Result<(), NoMem> {
        let phys_page = mem.alloc_pages_raw(1)?;
        virtmem::map(
            mem.get_inner_mut(),
            unsafe { self.root.as_mut().unwrap() },
            vaddr,
            phys_page as usize,
            EntryFlags::from_bits_truncate(flags as u64),
        );
        Ok(())
    }
}
