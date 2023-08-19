use super::{Capability, KernelAlloc};
use derivation_tree::caps::CapabilityIface;

pub use derivation_tree::caps::Memory;

#[derive(Copy, Clone)]
pub struct MemoryIface;
impl MemoryIface {
    pub(crate) fn create_init(
        mem: &mut Capability,
        alloc: KernelAlloc,
    ) -> Result<(), super::Error> {
        todo!()
    }
}

impl CapabilityIface<Capability> for MemoryIface {
    type InitArgs = usize;

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
