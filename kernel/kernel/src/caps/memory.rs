use crate::caps;
use crate::caps::errors::*;
use allocators::Arena;
use libkernel::mem::MemoryPage;

pub struct Memory {
    pub(crate) inner: Arena<'static, MemoryPage>,
}

impl Memory {
    pub fn init_sz(slot: &mut caps::CSlot, mem: &mut caps::CNode, pages: usize) -> Result<(), NoMem> {
        let memref = mem.get_memory_mut().unwrap();
        let ptr = memref.elem.alloc_pages_raw(pages)?;
        let slice = unsafe { core::slice::from_raw_parts_mut(ptr, pages) };
        let inner = Arena::new(slice);

        slot.set(Self { inner });
        unsafe { mem.link_derive(slot.cap.as_link()) };
        Ok(())
    }

    pub fn alloc_pages_raw(&mut self, pages: usize) -> Result<*mut MemoryPage, NoMem> {
        let alloc = self.inner.alloc_many_raw(pages).ok_or(NoMem)?;
        // TODO: Make this more safe. We only initialize this page later so just assuming that it is now initialized is clearly unsafe
        Ok(unsafe { core::mem::transmute(alloc) })
    }
}
