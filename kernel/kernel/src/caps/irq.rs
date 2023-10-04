use crate::caps::{Capability, Tag, Variant};
use core::mem::ManuallyDrop;
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
    type InitArgs = usize;

    fn init(&self, target: &mut impl AsStaticMut<Capability>, args: Self::InitArgs) {
        let target = target.as_static_mut();
        assert_eq!(target.tag, Tag::Uninit);

        target.tag = Tag::Irq;
        target.variant = Variant {
            irq: ManuallyDrop::new(Irq {
                interrupt_line: args,
            }),
        }
    }

    fn copy(&self, src: &impl AsStaticRef<Capability>, dst: &mut impl AsStaticMut<Capability>) {
        todo!()
    }

    fn destroy(&self, target: &mut Capability) {
        todo!()
    }
}
