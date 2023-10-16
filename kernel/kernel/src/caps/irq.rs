use crate::caps::{Capability, Tag, Uninit, Variant};
use core::mem::ManuallyDrop;
use derivation_tree::caps::CapabilityIface;
use derivation_tree::tree::TreeNodeOps;
use derivation_tree::{AsStaticMut, AsStaticRef, Correspondence};

/// An IRQ capability used for handling interrupts on a specific interrupt line
pub struct Irq {
    pub interrupt_line: usize,
}

impl Correspondence for Irq {
    fn corresponds_to(&self, other: &Self) -> bool {
        self.interrupt_line == other.interrupt_line
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
        let src = src.as_static_ref();
        let dst = dst.as_static_mut();
        assert_eq!(src.tag, Tag::Irq);
        assert_eq!(dst.tag, Tag::Uninit);

        {
            let src = src.get_inner_irq().unwrap();
            dst.tag = Tag::Irq;
            dst.variant.irq = ManuallyDrop::new(Irq {
                interrupt_line: src.interrupt_line,
            });
        }

        unsafe { src.insert_copy(dst) };
    }

    fn destroy(&self, target: &mut Capability) {
        assert_eq!(target.tag, Tag::Irq);
        if target.is_final_copy() {
            todo!("drop irq state (notification) in IrqControl");
        }

        target.tree_data.unlink();
        target.tag = Tag::Uninit;
        target.variant.uninit = Uninit {};
    }
}
