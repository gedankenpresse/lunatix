use crate::caps;
use crate::caps::errors::*;
use crate::mem::Page;

pub struct Memory {
    pub(crate) inner: memory::Arena<'static, crate::mem::Page>,
}

impl Memory {
    pub fn init_sz(mem: &mut caps::Memory, pages: usize) -> Result<caps::Cap<Self>, NoMem> {
        let ptr = mem.alloc_pages_raw(pages)?;
        let slice = unsafe { core::slice::from_raw_parts_mut(ptr, pages) };
        let inner = memory::Arena::new(slice);
        let cap = caps::Cap::from_content(Self { inner });
        Ok(cap)
    }

    pub fn alloc_pages_raw(&mut self, pages: usize) -> Result<*mut Page, NoMem> {
        let alloc = self.inner.alloc_many_raw(pages).ok_or(NoMem)?;
        // TODO: Make this more safe. We only initialize this page later so just assuming that it is now initialized is clearly unsafe
        Ok(unsafe { core::mem::transmute(alloc) })
    }
}
