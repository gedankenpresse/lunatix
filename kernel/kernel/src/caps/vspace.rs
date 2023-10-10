use core::mem::{ManuallyDrop, MaybeUninit};

use crate::caps::{self, Memory, Tag, Variant};
use crate::virtmem;
use allocators::Box;
use caps::errors::NoMem;
use derivation_tree::{caps::CapabilityIface, tree::TreeNodeOps, Correspondence};
use riscv::pt::{EntryFlags, PageTable};

// use crate::virtmem;

use super::Capability;

pub struct VSpace {
    pub(crate) root: *mut PageTable,
    pub(crate) asid: usize,
}

impl Correspondence for VSpace {
    fn corresponds_to(&self, other: &Self) -> bool {
        todo!("correspondence not implemented for vspace")
    }
}

impl VSpace {
    /// Allocate a range of virtual addresses
    /// Creates needed pages and page tables from given memory
    // TODO: fix usage of memory.get_inner
    pub(crate) fn map_range(
        &self,
        mem: &Capability,
        vaddr_base: usize,
        size: usize,
        flags: EntryFlags,
    ) -> Result<(), NoMem> {
        let mem = mem.get_inner_memory().unwrap();
        virtmem::map_range_alloc(
            &*mem.allocator,
            unsafe { self.root.as_mut().unwrap() },
            vaddr_base,
            size,
            flags | EntryFlags::Accessed | EntryFlags::Dirty,
        );
        Ok(())
    }

    /// Map the given physical address in this VSpace at the given virtual address.
    ///
    /// Missing intermediate page tables are automatically allocated from `mem`.
    pub(crate) fn map_address(
        &self,
        mem: &Memory,
        vaddr: usize,
        paddr: usize,
        flags: EntryFlags,
    ) -> Result<(), NoMem> {
        virtmem::map(
            &mem.allocator,
            unsafe { &mut *self.root },
            vaddr,
            paddr,
            flags | EntryFlags::Accessed | EntryFlags::Dirty,
        );
        Ok(())
    }
}

#[derive(Copy, Clone)]
pub struct VSpaceIface;

impl VSpaceIface {
    pub fn derive(&self, src: &Capability, target: &mut Capability) {
        assert_eq!(target.tag, Tag::Uninit);
        // TODO: make sure layout is the same
        let mut page: Box<MaybeUninit<PageTable>> =
            Box::new_uninit(&*src.get_inner_memory().unwrap().allocator).unwrap();
        PageTable::init_copy(page.as_mut_ptr().cast(), unsafe {
            crate::KERNEL_ROOT_PT
                .as_mapped()
                .raw()
                .as_ref()
                .expect("No Kernel Root Page Table found")
        });
        let page = unsafe { page.assume_init() };
        // save the capability into the target slot
        target.tag = Tag::VSpace;
        target.variant = Variant {
            vspace: ManuallyDrop::new(VSpace {
                root: page.leak() as *mut _,
                asid: 0,
            }),
        };

        unsafe {
            src.insert_derivation(target);
        }
    }
}

impl CapabilityIface<Capability> for VSpaceIface {
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
        let src = src.as_static_ref();
        let dst = dst.as_static_mut();
        assert_eq!(src.tag, Tag::VSpace);
        assert_eq!(dst.tag, Tag::Uninit);

        // semantically copy the vspace
        dst.tag = Tag::VSpace;
        {
            let src_vspace = src.get_inner_vspace().unwrap();
            dst.variant = Variant {
                vspace: ManuallyDrop::new(VSpace {
                    root: src_vspace.root,
                    asid: src_vspace.asid,
                }),
            }
        }

        // insert the new copy into the derivation tree
        unsafe {
            src.insert_copy(dst);
        }
    }

    fn destroy(&self, target: &mut Capability) {
        todo!()
    }
}
