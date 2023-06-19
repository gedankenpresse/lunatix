use crate::caps::errors::*;
use crate::caps;

pub struct CSpace {
    bits: usize,
    slots: *mut caps::CSlot,
}

fn cspace_pages(bits: usize) -> usize {
    use core::mem::size_of;
    const SLOTSIZE: usize = size_of::<caps::CSlot>();
    fn ceildiv(a: usize, b: usize) -> usize {
        return (a + b - 1) / b;
    }
    let nslot = 1 << bits;
    return ceildiv(nslot * SLOTSIZE, crate::mem::PAGESIZE);
}

impl CSpace {
    pub (crate) fn init_sz(mem: &mut caps::Memory, bits: usize) -> Result<caps::Cap<Self>, NoMem> {
        let pages = cspace_pages(bits);
        let slots = {
            let ptr = mem.alloc_pages_raw(pages)? as *mut caps::CSlot;
            let slots = unsafe { core::slice::from_raw_parts_mut(ptr, 1 << bits) };
            slots
        };
        for slot in slots.iter_mut() {
            *slot = caps::CSlot::default();
        }
        let cap= caps::Cap::from_content(Self { 
            bits,
            slots: slots.as_mut_ptr(),
        });
        Ok(cap)
    }

    pub (crate) fn get_slots(&self) -> &[caps::CSlot] {
        let nslots = 1 << self.bits;
        unsafe { core::slice::from_raw_parts(self.slots, nslots) }
    }

    pub (crate) fn get_slots_mut(&mut self) -> &mut [caps::CSlot] {
        let nslots = 1 << self.bits;
        unsafe { core::slice::from_raw_parts_mut(self.slots, nslots) }
    }

    pub (crate) fn get_slot(&self, slot: usize) -> Result<&caps::CSlot, InvalidCAddr> {
        self.get_slots().get(slot).ok_or(InvalidCAddr)
    }

    pub (crate) fn get_slot_mut(&mut self, slot: usize) -> Result<&mut caps::CSlot, InvalidCAddr> {
        self.get_slots_mut().get_mut(slot).ok_or(InvalidCAddr)
    }
}