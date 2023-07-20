//! The **Memory** capability

use crate::caps::cspace::CSpace;
use crate::caps::task::Task;
use crate::mem::{MemoryPage, PAGESIZE};
use allocators::Arena;
use core::mem;
use core::mem::MaybeUninit;

use allocators::ArenaAlloc;
/// The memory capability
///
/// This capability allows allocating memory for arbitrary usage.
pub struct Memory {
    allocator: Arena<'static, MemoryPage>,
}

impl Memory {
    pub fn new(allocator: Arena<'static, MemoryPage>) -> Self {
        Self { allocator }
    }

    pub unsafe fn alloc_bytes_raw(&mut self, size: usize) -> Option<*mut MaybeUninit<u8>> {
        let num_pages = if size % PAGESIZE == 0 {
            size / PAGESIZE
        } else {
            size / PAGESIZE + 1
        };

        let ptr = self.allocator.alloc_many(num_pages);
        if ptr.is_null() {
            None
        } else {
            Some(ptr.cast())
        }
    }

    pub unsafe fn alloc_pages_raw(&mut self, pages: usize) -> Option<*mut MaybeUninit<MemoryPage>> {
        let ptr = self.allocator.alloc_many(pages);
        if ptr.is_null() {
            None
        } else {
            Some(ptr.cast())
        }
    }

    pub unsafe fn derive_cspace(&mut self) -> Result<*mut CSpace, ()> {
        let ptr = self
            .alloc_bytes_raw(mem::size_of::<CSpace>())
            .ok_or(())?
            .cast();
        CSpace::init(ptr);
        Ok(ptr.cast())
    }

    pub unsafe fn derive_vspace(&mut self) -> () {
        todo!() // TODO Implement derive_vspace
    }

    pub unsafe fn derive_memory(&mut self) -> () {
        todo!() // TODO Implement derive_memory
    }

    pub unsafe fn derive_task(&mut self) -> Result<*mut Task, ()> {
        let ptr = self
            .alloc_bytes_raw(mem::size_of::<Task>())
            .ok_or(())?
            .cast();
        Task::init(ptr);
        Ok(ptr.cast())
    }
}
