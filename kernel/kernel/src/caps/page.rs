use super::Capability;
use crate::caps::{Tag, Variant};
use allocators::{AllocInit, Allocator};
use core::alloc::Layout;
use core::mem::ManuallyDrop;
use core::ptr;
use derivation_tree::tree::TreeNodeOps;
use derivation_tree::{caps::CapabilityIface, Correspondence};
use libkernel::mem;
use riscv::pt::MemoryPage;

/// A capability to physical memory.
pub struct Page {
    pub(crate) kernel_addr: *mut MemoryPage,
}

impl Correspondence for Page {
    fn corresponds_to(&self, other: &Self) -> bool {
        ptr::eq(self.kernel_addr, other.kernel_addr)
    }
}

#[derive(Copy, Clone)]
pub struct PageIface;

impl PageIface {
    /// Derive a page from a src memory by allocating one from it.
    /// The derived capability is then placed in `target`.
    pub fn derive(&self, src: &Capability, target: &mut Capability) {
        assert_eq!(src.tag, Tag::Memory);
        assert_eq!(target.tag, Tag::Uninit);

        let page = src
            .get_inner_memory()
            .unwrap()
            .allocator
            .allocate(Layout::new::<MemoryPage>(), AllocInit::Zeroed)
            .unwrap()
            .as_mut_ptr()
            .cast();

        // safe the capability into the target slot and insert it into the tree
        target.tag = Tag::Page;
        target.variant = Variant {
            page: ManuallyDrop::new(Page { kernel_addr: page }),
        };
        unsafe {
            src.insert_derivation(target);
        }
    }
}

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
