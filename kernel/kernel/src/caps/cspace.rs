use crate::caps::{CapCounted, KernelAlloc, Tag, Uninit, Variant};
use allocators::{AllocError, Box};
use core::cell::RefCell;
use core::mem;
use core::mem::ManuallyDrop;
pub use derivation_tree::caps::CapabilityIface;
use derivation_tree::tree::TreeNodeOps;
use derivation_tree::{AsStaticMut, AsStaticRef, Correspondence};
use syscall_abi::CAddr;

use super::Capability;

pub struct CSpace {
    pub slots: CapCounted<[RefCell<Capability>]>,
}

impl CSpace {
    /// Allocate enough memory from an allocator to hold the given number of slots and construct
    /// a CSpace from it
    pub fn alloc_new(
        allocator: &'static KernelAlloc,
        num_slots: usize,
    ) -> Result<Self, AllocError> {
        // this is necessary because otherwise CAddrs don't work correctly
        assert!(num_slots.is_power_of_two());

        // allocate memory and initialize it with default values
        let mut slots = Box::new_uninit_slice(num_slots, allocator)?;
        for slot in slots.iter_mut() {
            slot.write(RefCell::new(Default::default()));
        }

        // return result
        Ok(Self {
            slots: CapCounted::from_box(unsafe { slots.assume_init() }),
        })
    }

    /// Deallocate the backing memory of this CSpace.
    ///
    /// # Safety
    /// This method must only be called once and only on the last existing capability copy.
    pub unsafe fn deallocate(&mut self) {
        self.slots.destroy();
    }

    /// How many bits of a CAddr this CSpace requires to index all its slots.
    pub fn addr_bits(&self) -> usize {
        let mut len = self.slots.len();
        let mut n_shifts = 0;

        while len != 0 {
            len = len >> 1;
            n_shifts += 1;
        }

        n_shifts
    }

    /// Perform a lookup based on the given address and return a *TreeNode* if one corresponds to that address as well
    /// ass the remaining part of the CAddr.
    ///
    /// # Safety
    /// The returned node may not be linked into a derivation tree yet.
    ///
    /// Additionally, looking up a node from the cspace may produce overlapping aliases if the node is already part of
    /// a DerivationTree.
    unsafe fn lookup_raw(&self, addr: CAddr) -> Option<(*mut Capability, CAddr)> {
        let (slot, remainder) = addr.take_bits(self.addr_bits());
        let slot = self.slots.get(slot)?.as_ptr();
        Some((slot, remainder))
    }

    /// Fully resolve the given CAddr and return the capability that it points to.
    ///
    /// This function honors the hierarchical nature of CAddrs by recursing into child CSpaces.
    ///
    /// # Safety
    /// The returned node may not be linked into a derivation tree yet.
    ///
    /// Additionally, looking up a node from the CSpace may produce overlapping aliases if the node is already part of
    /// a derivation tree and must be selected via with a cursor before further uses.
    pub unsafe fn resolve_caddr(&self, addr: CAddr) -> Option<*mut Capability> {
        // TODO Properly use cursors
        let (slot_ptr, remainder) = self.lookup_raw(addr)?;
        let slot = unsafe { &mut *slot_ptr };
        if slot.tag == Tag::CSpace {
            let slot_cspace = slot.get_inner_cspace().unwrap();
            slot_cspace.resolve_caddr(remainder)
        } else {
            Some(slot_ptr)
        }
    }
}

impl Correspondence for CSpace {
    fn corresponds_to(&self, other: &Self) -> bool {
        let self_slots: &[_] = &self.slots;
        let other_slots: &[_] = &other.slots;
        self_slots.as_ptr() == other_slots.as_ptr()
    }
}

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
