use super::{asid::ASID_NONE, Capability, Error, Memory, Tag, VSpace, Variant};
use crate::virtmem::KernelMapper;

use allocators::{AllocInit, Allocator};
use core::{alloc::Layout, arch::asm, mem::ManuallyDrop, ptr};
use derivation_tree::{caps::CapabilityIface, tree::TreeNodeOps, Correspondence};
use libkernel::mem::PAGESIZE;
use riscv::{
    pt::{EntryFlags, MemoryPage},
    PhysMapper,
};
use syscall_abi::MapFlags;

/// A capability to physical memory.
pub struct Page {
    pub(crate) kernel_addr: *mut MemoryPage,
    pub(crate) asid: usize,
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
            page: ManuallyDrop::new(Page {
                kernel_addr: page,
                asid: 0,
            }),
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

pub fn map_page(
    page: &mut Page,
    mem: &Memory,
    vspace: &VSpace,
    flags: MapFlags,
    addr: usize,
) -> Result<(), Error> {
    // compute flags with which to map from arguments
    let mut entry_flags = EntryFlags::UserReadable;
    if flags.contains(MapFlags::READ) {
        entry_flags |= EntryFlags::Read;
    }
    if flags.contains(MapFlags::WRITE) {
        entry_flags |= EntryFlags::Write;
    }
    if flags.contains(MapFlags::EXEC) {
        entry_flags |= EntryFlags::Execute
    }

    // map the page
    assert_eq!(
        addr & !(PAGESIZE - 1),
        addr,
        "page address is not page-aligned"
    );

    if page.asid != ASID_NONE {
        return Err(Error::AlreadyMapped);
    }

    if vspace.asid == ASID_NONE {
        return Err(Error::NoAsid);
    }

    let paddr = unsafe { KernelMapper.mapped_to_phys(page.kernel_addr) } as usize;
    vspace.map_address(mem, addr, paddr, entry_flags)?;
    page.asid = vspace.asid;
    unsafe { asm!("sfence.vma") };
    Ok(())
}
