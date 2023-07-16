use libkernel::mem;

use crate::caps;

use super::{CapabilityInterface, Memory, Error};


/// A capability to physical memory.
pub struct Page {
    pub (crate) kernel_addr: *mut mem::MemoryPage,
}

impl Page {
    pub fn init(slot: &caps::CSlot, memslot: &caps::CSlot) -> Result<(), caps::Error> {
        memslot.derive(slot, |mem| {
            let memory_page = mem.alloc_pages_raw(1)?;
            let pagecap = Self { kernel_addr: memory_page };
            return Ok(pagecap.into());
        })
    }
}

#[derive(Copy, Clone)]
pub struct PageIface;

impl CapabilityInterface for PageIface {
    fn init(&self, slot: &caps::CSlot, mem: &mut Memory) -> Result<caps::Capability, Error> {
        todo!()
    }

    fn init_sz(&self, slot: &caps::CSlot, mem: &mut Memory, size: usize) -> Result<caps::Capability, Error>  {
        todo!()
    }

    fn destroy(&self, slot: &caps::CSlot) {
        todo!()
    }

    fn copy(&self, this: &caps::CSlot, target: &caps::CSlot) -> Result<(), Error> {
        todo!()
    }
}