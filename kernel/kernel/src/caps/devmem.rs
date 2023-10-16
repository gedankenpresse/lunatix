use core::{cell::RefCell, mem::ManuallyDrop};

use allocators::Box;
use derivation_tree::{caps::CapabilityIface, tree::TreeNodeOps};

use crate::caps::{Tag, Uninit, Variant};

use super::{CapCounted, Capability, KernelAlloc};

#[derive(Copy, Clone)]
pub struct DevmemEntry {
    pub base: usize,
    pub len: usize,
}

pub struct Devmem {
    pub inner_state: CapCounted<[RefCell<Option<DevmemEntry>>]>,
}

impl DevmemIface {
    pub(crate) fn create_init(
        &self,
        target_slot: &mut Capability,
        alloc: &'static KernelAlloc,
        devs: &[Option<DevmemEntry>],
    ) -> Result<(), super::Error> {
        assert_eq!(target_slot.tag, Tag::Uninit);
        let mut entries: Box<[RefCell<Option<DevmemEntry>>]> =
            Box::new_slice_with(devs.len(), alloc, |_| RefCell::new(None))
                .map_err(|_| super::Error::NoMem)?;
        for (entry, dev) in entries.iter_mut().zip(devs.iter()) {
            *entry.get_mut() = *dev;
        }

        let devmem = Devmem {
            inner_state: CapCounted::from_box(entries),
        };
        target_slot.tag = Tag::Devmem;
        target_slot.variant = Variant {
            devmem: ManuallyDrop::new(devmem),
        };
        Ok(())
    }
}

#[derive(Copy, Clone)]
pub struct DevmemIface;

impl CapabilityIface<Capability> for DevmemIface {
    type InitArgs = ();

    fn init(
        &self,
        target: &mut impl derivation_tree::AsStaticMut<Capability>,
        args: Self::InitArgs,
    ) {
        panic!("cant derive devmem")
    }

    fn copy(
        &self,
        src: &impl derivation_tree::AsStaticRef<Capability>,
        dst: &mut impl derivation_tree::AsStaticMut<Capability>,
    ) {
        let src = src.as_static_ref();
        let dst = dst.as_static_mut();
        assert_eq!(src.tag, Tag::Devmem);
        assert_eq!(dst.tag, Tag::Uninit);

        // semantically copy the cspace
        dst.tag = Tag::Devmem;
        {
            let src_mem = src.get_inner_devmem().unwrap();
            dst.variant = Variant {
                devmem: ManuallyDrop::new(Devmem {
                    inner_state: src_mem.inner_state.clone(),
                }),
            };
        }

        // insert the new copy into the derivation tree
        unsafe { src.insert_copy(dst) };
    }

    fn destroy(&self, target: &mut Capability) {
        assert_eq!(target.tag, Tag::Devmem);

        if target.is_final_copy() {
            todo!("return devmem memory");
        }

        target.tree_data.unlink();
        target.tag = Tag::Uninit;
        target.variant.uninit = Uninit {};
    }
}
