use crate::cap_counted::CapCounted;
use crate::correspondence::Correspondence;
use crate::tree::TreeNodeOps;
use allocators::{AllocError, Allocator, Box};
use core::cell::RefCell;

/// An address of a specific capability in a chain of CSpaces
pub type CAddr = usize;

/// A capability that is a handle to backing memory for [`TreeNodes`](TreeNode).
pub struct CSpace<'alloc, 'mem, A: Allocator<'mem>, T> {
    slots: CapCounted<'alloc, 'mem, A, [RefCell<T>]>,
}

impl<'alloc, 'mem, A: Allocator<'mem>, T: Default> CSpace<'alloc, 'mem, A, T> {
    /// Allocate enough memory from an allocator to hold the given number of slots and construct
    /// a CSpace from it
    pub fn alloc_new(allocator: &'alloc A, num_slots: usize) -> Result<Self, AllocError> {
        // this is necessary because otherwise CAddrs don't work correctly
        assert!(num_slots.is_power_of_two());

        // allocate memory and initialize it with default values
        let mut slots = Box::new_uninit_slice(num_slots, allocator)?;
        for slot in slots.iter_mut() {
            slot.write(RefCell::new(Default::default()));
        }

        // return result
        Ok(Self {
            slots: unsafe { slots.assume_init() }.into(),
        })
    }

    /// Deallocate the backing memory of this CSpace.
    ///
    /// # Safety
    /// This method must only be called once and only on the last existing capability copy.
    pub unsafe fn deallocate(&mut self) {
        self.slots.destroy();
    }

    /// Perform a lookup based on the given address and return a *TreeNode* if one corresponds to that address.
    ///
    /// # Safety
    /// The returned node may not be linked into a derivation tree yet.
    ///
    /// Additionally, looking up a node from the cspace may produce overlapping aliases if the node is already part of
    /// a DerivationTree.
    pub unsafe fn lookup_raw(&self, addr: CAddr) -> Option<*mut T> {
        Some(self.slots.get(addr)?.as_ptr())
    }
}

impl<'mem, A: Allocator<'mem>, T: TreeNodeOps> Correspondence for CSpace<'_, 'mem, A, T> {
    fn corresponds_to(&self, other: &Self) -> bool {
        let self_slots: &[_] = &self.slots;
        let other_slots: &[_] = &other.slots;
        self_slots.as_ptr() == other_slots.as_ptr()
    }
}
