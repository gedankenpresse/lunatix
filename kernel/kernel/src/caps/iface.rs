use super::{CSlot, Memory, Capability, Error, Variant};

pub trait CapabilityInterface {
    fn init(&self, slot: &CSlot, mem: &mut Memory) -> Result<Capability, Error>;
    fn init_sz(&self, slot: &CSlot, mem: &mut Memory, size: usize) -> Result<Capability, Error> ;
    fn destroy(&self, slot: &CSlot);
    fn copy(&self, this: &CSlot, target: &CSlot) -> Result<(), Error>;
}


#[derive(Copy, Clone)]
pub struct UninitIface;

impl CapabilityInterface for UninitIface {
    fn init(&self, slot: &CSlot, mem: &mut Memory) -> Result<Capability, Error> {
        todo!()
    }

    fn init_sz(&self, slot: &CSlot, mem: &mut Memory, size: usize) -> Result<Capability, Error>  {
        todo!()
    }

    fn destroy(&self, slot: &CSlot) {
        todo!()
    }

    fn copy(&self, this: &CSlot, target: &CSlot) -> Result<(), Error> {
        todo!()
    }
}

impl CapabilityInterface for Variant {
    fn init(&self, slot: &CSlot, mem: &mut Memory) -> Result<Capability, Error> {
        self.as_iface().init(slot, mem)
    }

    fn init_sz(&self, slot: &CSlot, mem: &mut Memory, size: usize) -> Result<Capability, Error>  {
        self.as_iface().init_sz(slot, mem, size)
    }

    fn destroy(&self, slot: &CSlot) {
        self.as_iface().destroy(slot)
    }

    fn copy(&self, this: &CSlot, target: &CSlot) -> Result<(), Error> {
        self.as_iface().copy(this, target)
    }
}


impl Variant {
    pub (super) fn as_iface(&self) -> &dyn CapabilityInterface {
        match self {
            Variant::Uninit(iface) => iface,
            Variant::Memory(iface) => iface,
            Variant::CSpace(iface) => iface,
            Variant::VSpace(iface) => iface,
            Variant::Task(iface) => iface,
            Variant::Page(iface) => iface,
        }
    } 
}