use libkernel::mem;

use crate::caps;


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