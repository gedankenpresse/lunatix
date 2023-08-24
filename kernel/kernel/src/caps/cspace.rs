use crate::caps::{Tag, Variant};
use core::mem;
use core::mem::ManuallyDrop;
pub use derivation_tree::caps::CapabilityIface;
use derivation_tree::tree::TreeNodeOps;
use derivation_tree::{AsStaticMut, AsStaticRef};

use super::Capability;

pub type CSpace = derivation_tree::caps::CSpace<'static, 'static, Capability>;

#[derive(Copy, Clone)]
pub struct CSpaceIface;
impl CSpaceIface {
    pub fn derive(&self, src_mem: &Capability, target_slot: &mut Capability, num_slots: usize) {
        assert_eq!(target_slot.tag, Tag::Uninit);

        // create a new cspace which is allocated from src_mem
        let cspace = derivation_tree::caps::CSpace::alloc_new(
            &*src_mem.get_inner_memory().unwrap().allocator,
            num_slots,
        )
        .unwrap();

        // Safety: it is safe to ignore lifetimes for this CSoace, because the derivation tree ensures correct lifetimes at runtime
        let cspace = unsafe {
            mem::transmute::<derivation_tree::caps::CSpace<'_, '_, Capability>, CSpace>(cspace)
        };

        // save the capability into the target slot
        target_slot.tag = Tag::CSpace;
        target_slot.variant = Variant {
            cspace: ManuallyDrop::new(cspace),
        };
        unsafe {
            src_mem.insert_derivation(target_slot);
        }
    }
}

impl CapabilityIface<Capability> for CSpaceIface {
    type InitArgs = usize;

    fn init(&self, target: &mut impl AsStaticMut<Capability>, args: Self::InitArgs) {
        todo!()
    }

    fn copy(&self, src: &impl AsStaticRef<Capability>, dst: &mut impl AsStaticMut<Capability>) {
        assert_eq!(src.as_static_ref().tag, Tag::CSpace);
        assert_eq!(dst.as_static_ref().tag, Tag::Uninit);

        // semantically copy the cspace
        dst.as_static_mut().tag = Tag::CSpace;
        dst.as_static_mut().variant = Variant {
            cspace: ManuallyDrop::new(CSpace {
                slots: unsafe { &src.as_static_ref().variant.cspace }.slots.clone(),
            }),
        };

        // insert the new copy into the derivation tree
        unsafe { src.as_static_ref().insert_copy(dst.as_static_mut()) };
    }

    fn destroy(&self, target: &mut Capability) {
        todo!()
    }
}
