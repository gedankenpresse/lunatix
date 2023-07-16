use core::cell::RefCell;

use crate::caps::{self, Variant, CapabilityInterface};
use crate::caps::errors::*;
use allocators::Arena;
use libkernel::mem::MemoryPage;

/// A capability to physical memory.
pub struct Memory {

    /// This is the (allocator) for the backing memory.
    /// It returns pages in kernel space.
    /// When mapping to userspace, these have to be converted using kernel_to_phys first
    /// 
    /// This doesn't have to be a pointer anymore because derivation tree itself uses references
    pub(crate) inner: *mut RefCell<Arena<'static, MemoryPage>>,
}

impl Memory {
    pub fn create_init(mut alloc: Arena<'static, MemoryPage>) -> Self {
        let state: *mut RefCell<Arena<MemoryPage>> = alloc.alloc_one_raw().unwrap().cast();
        unsafe { (*state) = RefCell::new(alloc); }
        Memory { inner: state }
    }

    pub fn init_sz(slot: &caps::CSlot, mem: &caps::CSlot, pages: usize) -> Result<(), caps::Error> {
        mem.derive(slot, |mem| {
            let state: *mut RefCell<Arena<MemoryPage>> = mem.alloc_pages_raw(1)?.cast();
            let ptr = mem.alloc_pages_raw(pages)?;
            let slice = unsafe { core::slice::from_raw_parts_mut(ptr, pages) };
            
            unsafe { 
                (*state) = RefCell::new(Arena::new(slice));
            }
            let me= Memory {
                inner: state,
            };
            return Ok(me.into());
        })
    }

    // TODO: this should be private
    pub (super) fn get_inner_mut(&mut self) -> &mut Arena<'static, MemoryPage> {
        unsafe { self.inner.as_mut().unwrap().get_mut() }
    }

    // TODO: this should be private
    pub (super) fn get_inner(&self) -> &RefCell<Arena<'static, MemoryPage>> {
        unsafe { self.inner.as_ref().unwrap() }
    }

    pub fn alloc_pages_raw(&mut self, pages: usize) -> Result<*mut MemoryPage, NoMem> {
        let alloc = self.get_inner_mut().alloc_many_raw(pages).ok_or(NoMem)?;
        // TODO: Make this more safe. We only initialize this page later so just assuming that it is now initialized is clearly unsafe
        Ok(unsafe { core::mem::transmute(alloc) })
    }

    pub fn copy(this: &mut caps::CSlot, other: &caps::CSlot) -> Result<(), caps::Error> {
        assert!(other.is_uninit());
        this.cap.copy_link(&other.cap);
        this.cap.copy_value(&other.cap);
        Ok(())
    }

    pub fn send(mem: &caps::CSlot, label: usize, caps: &[Option<&caps::CSlot>], params: &[usize]) -> Result<usize, caps::Error> {
        log::debug!("label: {label}, num_caps: {}, params: {params:?}", caps.len());
        const ALLOC: usize = 0;
        match label {
            ALLOC => {
                if caps.len() != 1 {
                    return Err(caps::Error::InvalidArg);
                }

                let target_slot = caps[0].unwrap();
                match params.len() {
                    1 => {
                        assert!(target_slot.is_uninit());
                        let variant = Variant::try_from(params[0])?;
                        mem.derive(target_slot, |mem| {
                            variant.as_iface().init(target_slot, mem)
                        })?;
                    },
                    2 => {
                        assert!(target_slot.is_uninit());
                        let variant = Variant::try_from(params[0])?;
                        mem.derive(target_slot, |mem| {
                            variant.as_iface().init_sz(target_slot, mem, params[1])
                        })?;
                    },
                    _ => return Err(caps::Error::InvalidArg),
                }

                return Ok(0);
            },
            _ => Err(caps::Error::InvalidOp)
        }
    }
}

#[derive(Copy, Clone)]
pub struct MemoryIface;

impl CapabilityInterface for MemoryIface {
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

