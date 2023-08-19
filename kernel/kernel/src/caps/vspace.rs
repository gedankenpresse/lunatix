use crate::caps;
use caps::errors::NoMem;
use derivation_tree::caps::CapabilityIface;
use riscv::pt::{EntryFlags, PageTable};

use crate::virtmem;

use super::Capability;

pub struct VSpace {
    pub(crate) root: *mut PageTable,
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
