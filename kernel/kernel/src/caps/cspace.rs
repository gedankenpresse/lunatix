use crate::caps::{Tag, Uninit, Variant};
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

    fn init(&self, _target: &mut impl AsStaticMut<Capability>, _args: Self::InitArgs) {
        todo!()
    }

    fn copy(&self, src: &impl AsStaticRef<Capability>, dst: &mut impl AsStaticMut<Capability>) {
        let src = src.as_static_ref();
        let dst = dst.as_static_mut();
        assert_eq!(src.tag, Tag::CSpace);
        assert_eq!(dst.tag, Tag::Uninit, "destination is not uninit");

        // semantically copy the cspace
        dst.tag = Tag::CSpace;
        dst.variant = Variant {
            cspace: ManuallyDrop::new(CSpace {
                slots: unsafe { &src.variant.cspace }.slots.clone(),
            }),
        };

        // insert the new copy into the derivation tree
        unsafe { src.insert_copy(dst) };
    }

    fn destroy(&self, target: &mut Capability) {
        // TODO: handle recursive cspaces
        assert_eq!(target.tag, Tag::CSpace);

        if target.is_final_copy() {
            let _cspace = target.get_inner_cspace_mut().unwrap();
            todo!("destroy cspace slots and dealloc cspace");
        }

        target.tree_data.unlink();
        target.tag = Tag::Uninit;
        target.variant.uninit = Uninit {};
    }
}
