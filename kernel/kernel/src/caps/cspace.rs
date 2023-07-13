use core::cell::RefCell;

use crate::caps;
use crate::caps::errors::*;
use crate::caps::CSlot;
use libkernel::mem::PAGESIZE;

pub struct CSpace {
    bits: usize,
    slots: *mut RefCell<caps::CSlot>,
}

fn cspace_pages(bits: usize) -> usize {
    use core::mem::size_of;
    const SLOTSIZE: usize = size_of::<caps::CSlot>();
    fn ceildiv(a: usize, b: usize) -> usize {
        return (a + b - 1) / b;
    }
    let nslot = 1 << bits;
    return ceildiv(nslot * SLOTSIZE, PAGESIZE);
}

impl CSpace {
    pub(crate) fn init_sz(slot: &mut caps::CSlot, mem: &mut caps::CNode, bits: usize) -> Result<(), NoMem> {
        let memref = mem.get_memory_mut().unwrap();
        let pages = cspace_pages(bits);
        let slots = {
            let ptr = memref.elem.alloc_pages_raw(pages)? as *mut RefCell<caps::CSlot>;
            for i in 0..(1 << bits) {
                unsafe { *ptr.add(i) = RefCell::new(CSlot::empty()) }
            }
            let slots = unsafe { core::slice::from_raw_parts_mut(ptr, 1 << bits) };
            slots
        };

        slot.set(Self { bits, slots: slots.as_mut_ptr() }).unwrap();
        unsafe { mem.link_derive(slot.cap.as_link()) };

        Ok(())
    }

    /// This function looks up capabilities in cspaces.
    /// If we want to keep close to seL4 behaviour, we should recursively lookup caps.
    /// 
    /// TODO: fix interior mutability/aliasing
    /// This should only return non-aliasing cslots, and actually have &mut type, but
    /// that's not possible without rc, which I'm not going to bother with right now...
    pub(crate) fn lookup(&self, cap: usize) -> Result<&RefCell<caps::CSlot>, InvalidCAddr> {
        let mutself: &mut Self = unsafe { (self as *const Self as *mut Self).as_mut().unwrap() };
        let slot = mutself.get_slot(cap)?;
        return Ok(slot);
    }

    pub(crate) fn get_slots(&self) -> &[RefCell<caps::CSlot>] {
        let nslots = 1 << self.bits;
        unsafe { core::slice::from_raw_parts(self.slots, nslots) }
    }

    pub(crate) fn get_slot(&self, slot: usize) -> Result<&RefCell<caps::CSlot>, InvalidCAddr> {
        self.get_slots().get(slot).ok_or(InvalidCAddr)
    }
}
