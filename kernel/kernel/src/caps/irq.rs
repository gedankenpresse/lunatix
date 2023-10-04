use crate::caps::Capability;
use derivation_tree::caps::CapabilityIface;
use derivation_tree::{AsStaticMut, AsStaticRef, Correspondence};

/// An IRQ capability used for handling interrupts on a specific interrupt line
pub struct Irq {
    pub interrupt_line: usize,
}

impl Correspondence for Irq {
    fn corresponds_to(&self, other: &Self) -> bool {
        todo!()
    }
}

/// The interface for interacting with IRQ capabilities
#[derive(Copy, Clone)]
pub struct IrqIface;

impl CapabilityIface<Capability> for IrqIface {
    type InitArgs = ();

    fn init(&self, target: &mut impl AsStaticMut<Capability>, args: Self::InitArgs) {
        todo!()
    }

    fn copy(&self, src: &impl AsStaticRef<Capability>, dst: &mut impl AsStaticMut<Capability>) {
        todo!()
    }

    fn destroy(&self, target: &mut Capability) {
        todo!()
    }
}
