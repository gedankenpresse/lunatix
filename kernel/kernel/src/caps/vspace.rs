use core::mem::MaybeUninit;

use crate::caps;
use caps::errors::NoMem;
use riscv::pt::{EntryFlags, MemoryPage, PageTable};

use crate::virtmem;

use super::{CapabilityInterface, Error, Memory};

pub struct VSpace {
    pub(crate) root: *mut PageTable,
}

impl VSpace {
    /// Allocate a range of virtual addresses
    /// Creates needed pages and page tables from given memory
    // TODO: fix usage of memory.get_inner
    pub(crate) fn map_range(
        &self,
        mem: &caps::CSlot,
        vaddr_base: usize,
        size: usize,
        flags: usize,
    ) -> Result<(), NoMem> {
        let mut memref = mem.get_memory_mut().unwrap();
        log::debug!("map range, root: {:p}", self.root);
        virtmem::map_range_alloc(
            memref.get_inner_mut(),
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

#[derive(Copy, Clone)]
pub struct VspaceIface;

impl CapabilityInterface for VspaceIface {
    fn init(&self, slot: &caps::CSlot, mem: &mut Memory) -> Result<caps::Capability, Error> {
        let ptpage = mem.alloc_pages_raw(1)?;
        let root = PageTable::init_copy(ptpage.cast::<MaybeUninit<MemoryPage>>(), unsafe {
            crate::KERNEL_ROOT_PT
                .as_mapped()
                .raw()
                .as_ref()
                .expect("No Kernel Root Page Table found")
        });
        return Ok(VSpace { root }.into());
    }

    fn init_sz(
        &self,
        slot: &caps::CSlot,
        mem: &mut Memory,
        size: usize,
    ) -> Result<caps::Capability, Error> {
        todo!()
    }

    fn destroy(&self, slot: &caps::CSlot) {
        todo!()
    }

    fn copy(&self, this: &caps::CSlot, target: &caps::CSlot) -> Result<(), Error> {
        todo!()
    }
}
