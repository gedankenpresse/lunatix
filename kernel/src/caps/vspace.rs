use core::mem::MaybeUninit;

use crate::{caps, mem};
use caps::errors::NoMem;

use crate::virtmem;

pub struct VSpace {
    pub(crate) root: *mut virtmem::PageTable,
}

impl VSpace {
    pub(crate) fn init(mem: &mut caps::Memory) -> Result<caps::Cap<Self>, NoMem> {
        let root = mem.alloc_pages_raw(1)?;
        let root = virtmem::PageTable::init_copy(root.cast::<MaybeUninit<mem::Page>>(), unsafe {
            crate::KERNEL_ROOT_PT
                .as_ref()
                .expect("No Kernel Root Page Table found")
        });
        let cap = caps::Cap::from_content(Self { root });
        Ok(cap)
    }

    // Allocate a range of virtual addresses
    // Creates needed pages and page tables from given memory
    pub(crate) fn map_range(
        &self,
        mem: &mut caps::Memory,
        vaddr_base: usize,
        size: usize,
        flags: usize,
    ) -> Result<(), NoMem> {
        virtmem::map_range_alloc(
            &mut mem.inner,
            unsafe { self.root.as_mut().unwrap() },
            vaddr_base,
            size,
            flags,
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
            &mut mem.inner,
            unsafe { self.root.as_mut().unwrap() },
            vaddr,
            phys_page as usize,
            flags,
        );
        Ok(())
    }
}