use super::{CSlot, Capability, Error, Memory, Variant};

/// A Capability has to implement this Interface.
/// The basic operations of an capability are intialization, copying and destruction.
///
/// For these purposes, we provide the init/init_sz, copy, and destroy operations.
/// It is expected that you create a zero-sized type that implements these operations.
/// If initialization requires some extra size arguments, implement init_sz instead of init.
///
/// It is not expected that the Capability Struct itself implements this interface.
/// This is because operations like copy and destroy require access to the links in the derivation tree.
/// With the current design, these links reside not in the Capability Variant itself, but in the CSlot.
/// Should be change the Capability Tree to be an intrusive collection, we can change this requirement.
pub trait CapabilityInterface {
    /// Initialize this capability into the given slot.
    ///
    /// The Caller does *not* have to care about linking the derivation from mem.
    /// In fact, they can't, because they lack access to the link pointers of the Memory cap.
    ///
    /// TODO: decide if the caller can assume that the CSlot is uninitialized, or if you have to check and abort.
    fn init(&self, slot: &CSlot, mem: &mut Memory) -> Result<Capability, Error>;

    /// Initialize this capability into the given slot using a size parameter.
    ///
    /// If you don't require a size to initialize the cap, return `Error::Unsupported` instead.
    ///
    /// The Caller does *not* have to care about linking the derivation from mem.
    /// In fact, they can't, because they lack access to the link pointers of the Memory cap.
    ///
    /// TODO: decide if the caller can assume that the CSlot is uninitialized, or if you have to check and abort.
    fn init_sz(&self, slot: &CSlot, mem: &mut Memory, size: usize) -> Result<Capability, Error>;

    /// Destroy this Capability.
    ///
    /// If this is the last copy of this capability, this means you have to free any
    /// underlying allocations, if any.
    ///
    /// Destroying a capability should never fail, therefore you can't return an Error from this function :)
    ///
    /// After calling destroy on a CSlot, it must be in Uninit state.
    fn destroy(&self, slot: &CSlot);

    /// Create a copy of this Capability in the target `CSlot`.
    ///
    /// TODO: decide if the caller can assume that the target CSlot is uninitialized, or if you have to check and abort.
    fn copy(&self, this: &CSlot, target: &CSlot) -> Result<(), Error>;
}

#[derive(Copy, Clone)]
pub struct UninitIface;

impl CapabilityInterface for UninitIface {
    fn init(&self, slot: &CSlot, mem: &mut Memory) -> Result<Capability, Error> {
        Err(Error::Unsupported)
    }

    fn init_sz(&self, slot: &CSlot, mem: &mut Memory, size: usize) -> Result<Capability, Error> {
        Err(Error::Unsupported)
    }

    fn destroy(&self, slot: &CSlot) {}

    fn copy(&self, this: &CSlot, target: &CSlot) -> Result<(), Error> {
        Err(Error::Unsupported)
    }
}

impl CapabilityInterface for Variant {
    fn init(&self, slot: &CSlot, mem: &mut Memory) -> Result<Capability, Error> {
        self.as_iface().init(slot, mem)
    }

    fn init_sz(&self, slot: &CSlot, mem: &mut Memory, size: usize) -> Result<Capability, Error> {
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
    pub fn as_iface(&self) -> &dyn CapabilityInterface {
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
