use core::mem::{ManuallyDrop, MaybeUninit};

use crate::caps::{self, Tag, Variant};
use allocators::Box;
use caps::errors::NoMem;
use derivation_tree::{caps::CapabilityIface, tree::TreeNodeOps, Correspondence};
use riscv::pt::PageTable;

// use crate::virtmem;

use super::Capability;

pub struct VSpace {
    pub(crate) root: *mut PageTable,
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
        flags: usize,
    ) -> Result<(), NoMem> {
        let memref = mem.get_memory().unwrap().as_ref();
        log::debug!("map range, root: {:p}", self.root);
        todo!();
        /*
        virtmem::map_range_alloc(
            memref.get_inner_mut(),
            unsafe { self.root.as_mut().unwrap() },
            vaddr_base,
            size,
            EntryFlags::from_bits_truncate(flags as u64),
        );
        Ok(())
        */
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
        todo!()
    }

    fn destroy(&self, target: &mut Capability) {
        todo!()
    }
}
