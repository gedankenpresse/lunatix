use core::cell::RefCell;

use crate::caps;
use crate::caps::errors::*;
use crate::caps::CSlot;
use libkernel::mem::PAGESIZE;

pub struct CSpace {
    bits: usize,
    // TODO: remove double RefCell
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
    pub(crate) fn init_sz(slot: &mut caps::CSlot, mem: &caps::CSlot, bits: usize) -> Result<(), caps::Error> {
        mem.derive(slot, |mem| {
            let pages = cspace_pages(bits);
            let slots = {
                let ptr = mem.alloc_pages_raw(pages)? as *mut RefCell<caps::CSlot>;
                for i in 0..(1 << bits) {
                    unsafe { *ptr.add(i) = RefCell::new(CSlot::empty()) }
                }
                let slots = unsafe { core::slice::from_raw_parts_mut(ptr, 1 << bits) };
                slots
            };
            let cspace = Self { bits, slots: slots.as_mut_ptr() };
            return Ok(cspace.into());
        })
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

    pub(crate) fn copy(this: &caps::CSlot, other: &caps::CSlot) -> Result<(), caps::Error> {
        assert_eq!(other.get_variant() as usize, caps::Variant::Uninit as usize);
        this.cap.copy_link(&other.cap);
        this.cap.copy_value(&other.cap);
        Ok(())
    }
}
