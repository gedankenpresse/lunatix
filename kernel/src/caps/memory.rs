use crate::caps;
use crate::caps::errors::*;
use crate::mem::Page;

pub struct Memory {
    pub(crate) inner: memory::Memory<'static, crate::mem::Page>,
}

impl Memory {
    pub(crate) fn init_sz(mem: &mut caps::Memory, pages: usize) -> Result<caps::Cap<Self>, NoMem> {
        let ptr = mem.alloc_pages_raw(pages)?;
        let slice = unsafe { core::slice::from_raw_parts_mut(ptr, pages) };
        let inner = memory::Memory::new(slice);
        let cap = caps::Cap::from_content(Self { inner });
        Ok(cap)
    }

    pub fn alloc_pages_raw(&mut self, pages: usize) -> Result<*mut Page, NoMem> {
        self.inner.alloc_many_raw(pages).ok_or(NoMem)
    }
}
