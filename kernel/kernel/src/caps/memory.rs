use core::cell::RefCell;
use core::ops::DerefMut;

use crate::caps::{self, Variant};
use crate::caps::errors::*;
use allocators::Arena;
use libkernel::mem::MemoryPage;

use super::Capability;

/// A capability to physical memory.
pub struct Memory {

    /// This is the (allocator) for the backing memory.
    /// It returns pages in kernel space.
    /// When mapping to userspace, these have to be converted using kernel_to_phys first
    pub(crate) inner: *mut RefCell<Arena<'static, MemoryPage>>,
}

impl Memory {
    pub fn create_init(mut alloc: Arena<'static, MemoryPage>) -> Self {
        let state: *mut RefCell<Arena<MemoryPage>> = alloc.alloc_one_raw().unwrap().cast();
        unsafe { (*state) = RefCell::new(alloc); }
        Memory { inner: state }
    }

    pub fn init_sz(slot: &mut caps::CSlot, mem: &mut caps::CNode, pages: usize) -> Result<(), NoMem> {
        let memref = mem.get_memory_mut().unwrap();
        let state: *mut RefCell<Arena<MemoryPage>> = memref.elem.alloc_pages_raw(1)?.cast();
        let ptr = memref.elem.alloc_pages_raw(pages)?;
        let slice = unsafe { core::slice::from_raw_parts_mut(ptr, pages) };
        
        unsafe { 
            (*state) = RefCell::new(Arena::new(slice));
        }
        let me= Memory {
            inner: state,
        };
        slot.set(me).unwrap();
        unsafe { mem.link_derive(slot.cap.as_link()) };
        Ok(())
    }

    // TODO: this should be private
    pub fn get_inner_mut(&mut self) -> &mut Arena<'static, MemoryPage> {
        unsafe { self.inner.as_mut().unwrap().get_mut() }
    }

    // TODO: this should be private
    pub fn get_inner(&self) -> &RefCell<Arena<'static, MemoryPage>> {
        unsafe { self.inner.as_ref().unwrap() }
    }

    pub fn alloc_pages_raw(&mut self, pages: usize) -> Result<*mut MemoryPage, NoMem> {
        let alloc = self.get_inner_mut().alloc_many_raw(pages).ok_or(NoMem)?;
        // TODO: Make this more safe. We only initialize this page later so just assuming that it is now initialized is clearly unsafe
        Ok(unsafe { core::mem::transmute(alloc) })
    }

    pub fn copy(this: &mut caps::CNode, other: &mut caps::CNode) -> Result<(), caps::Error> {
        assert_eq!(other.elem.get_variant() as usize, Variant::Uninit as usize);
        other.elem = Capability::Memory(Memory { inner: this.get_memory_mut()?.elem.inner });
        unsafe { this.link_copy(other.as_link()) };
        Ok(())
    }

    pub fn send(mem: &mut caps::CNode, label: usize, caps: &[Option<&RefCell<caps::CSlot>>], params: &[usize]) -> Result<usize, caps::Error> {
        const ALLOC: usize = 0;
        match label {
            ALLOC => {
                if caps.len() != 1 {
                    return Err(caps::Error::InvalidArg);
                }
                if params.len() < 1 {
                    return Err(caps::Error::InvalidArg);
                }

                let mut target_slot = caps[0].as_ref().unwrap().try_borrow_mut()?;
                let captype = params[0];
                let size = params.get(1).copied().unwrap_or(0);

                alloc_impl(mem, target_slot.deref_mut(), captype, size)
            },
            _ => Err(caps::Error::InvalidOp)
        }
    }
}

fn alloc_impl(
    mem: &mut caps::Node<caps::Capability>,
    target_slot: &mut caps::CSlot,
    captype: usize,
    size: usize
) -> Result<usize, Error> {
    todo!()
}

