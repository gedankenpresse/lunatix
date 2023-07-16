use crate::caps;
use crate::caps::errors::*;
use crate::caps::CSlot;
use libkernel::mem::PAGESIZE;

use super::Capability;
use super::CapabilityInterface;

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
    return ceildiv(nslot * SLOTSIZE, PAGESIZE);
}

impl CSpace {
    /// This function looks up capabilities in cspaces.
    /// If we want to keep close to seL4 behaviour, we should recursively lookup caps.
    pub(crate) fn lookup(&self, cap: usize) -> Result<&caps::CSlot, InvalidCAddr> {
        let slot = self.get_slot(cap)?;
        return Ok(slot);
    }

    fn get_slots(&self) -> &[caps::CSlot] {
        let nslots = 1 << self.bits;
        unsafe { core::slice::from_raw_parts(self.slots, nslots) }
    }

    fn get_slot(&self, slot: usize) -> Result<&caps::CSlot, InvalidCAddr> {
        self.get_slots().get(slot).ok_or(InvalidCAddr)
    }
}

#[derive(Copy, Clone)]
pub struct CSpaceIface;

impl CapabilityInterface for CSpaceIface {
    fn init_sz(
        &self,
        slot: &caps::CSlot,
        mem: &mut caps::Memory,
        bits: usize,
    ) -> Result<Capability, caps::Error> {
        let pages = cspace_pages(bits);
        let slots = {
            let ptr = mem.alloc_pages_raw(pages)? as *mut caps::CSlot;
            for i in 0..(1 << bits) {
                unsafe { *ptr.add(i) = CSlot::empty() }
            }
            let slots = unsafe { core::slice::from_raw_parts_mut(ptr, 1 << bits) };
            slots
        };
        let cspace = CSpace {
            bits,
            slots: slots.as_mut_ptr(),
        };
        return Ok(cspace.into());
    }
    fn init(&self, slot: &CSlot, mem: &mut caps::Memory) -> Result<Capability, Error> {
        return Err(Error::InvalidOp);
    }

    fn destroy(&self, slot: &CSlot) {
        todo!()
    }

    fn copy(&self, this: &caps::CSlot, other: &caps::CSlot) -> Result<(), caps::Error> {
        assert!(other.is_uninit());
        this.cap.copy_link(&other.cap);
        this.cap.copy_value(&other.cap);
        Ok(())
    }
}
