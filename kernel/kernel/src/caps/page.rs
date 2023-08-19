use super::Capability;
use derivation_tree::caps::CapabilityIface;
use libkernel::mem;

/// A capability to physical memory.
pub struct Page {
    pub(crate) kernel_addr: *mut mem::MemoryPage,
}

#[derive(Copy, Clone)]
pub struct PageIface;

impl CapabilityIface<Capability> for PageIface {
    type InitArgs = ();

    fn init(
        &self,
        target: &mut impl derivation_tree::AsStaticMut<Capability>,
        args: Self::InitArgs,
    ) {
        todo!()
    }

    fn copy(
        &self,
        src: &impl derivation_tree::AsStaticRef<Capability>,
        dst: &mut impl derivation_tree::AsStaticMut<Capability>,
    ) {
        todo!()
    }

    fn destroy(&self, target: &mut Capability) {
        todo!()
    }
}
