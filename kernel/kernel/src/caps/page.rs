use super::{
    asid::{ASID_NONE, ASID_POOL},
    Capability, Error, Memory, Tag, VSpace, Variant,
};
use crate::{caps::Uninit, virtmem::KernelMapper};

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
    pub(crate) vaddr: *mut u8,
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
                vaddr: core::ptr::null_mut(),
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
        let src = src.as_static_ref();
        let dst = dst.as_static_mut();
        assert_eq!(src.tag, Tag::Page);
        assert_eq!(dst.tag, Tag::Uninit);

        {
            let src = src.get_inner_page().unwrap();
            dst.tag = Tag::Page;
            dst.variant.page = ManuallyDrop::new(Page {
                kernel_addr: src.kernel_addr,
                vaddr: core::ptr::null_mut(),
                asid: ASID_NONE,
            });
        }
        unsafe { src.insert_copy(dst) };
    }

    fn destroy(&self, target: &mut Capability) {
        assert_eq!(target.tag, Tag::Page);
        let kernel_addr = {
            let page = target.get_inner_page_mut().unwrap();
            page.unmap();
            page.kernel_addr
        };
        if target.is_final_copy() {
            let Some(parent) = (unsafe { target.get_parent().as_ref() }) else {
                panic!("page has no parent");
            };
            assert_eq!(parent.tag, Tag::Memory);
            let parent = parent.get_inner_memory().unwrap();
            unsafe {
                parent
                    .allocator
                    .deallocate(kernel_addr as *mut u8, Layout::new::<MemoryPage>())
            };
        }
        target.tree_data.unlink();
        target.tag = Tag::Uninit;
        target.variant.uninit = Uninit {};
    }
}

impl Page {
    pub fn unmap(&mut self) {
        let mut page = self;
        if page.asid == ASID_NONE {
            return;
        }
        let Ok(asid) = (unsafe { ASID_POOL.find_asid(page.asid) }) else {
            return;
        };
        let pt = unsafe { asid.pt.as_mut().unwrap() };
        riscv::pt::unmap(KernelMapper, pt, page.vaddr as usize, unsafe {
            KernelMapper.mapped_to_phys(page.kernel_addr) as usize
        });
        page.asid = ASID_NONE;
        page.vaddr = core::ptr::null_mut();
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
        log::error!("no asid!");
        return Err(Error::NoAsid);
    }

    let paddr = unsafe { KernelMapper.mapped_to_phys(page.kernel_addr) } as usize;
    vspace.map_address(mem, addr, paddr, entry_flags)?;
    page.asid = vspace.asid;
    page.vaddr = addr as *mut u8;
    unsafe { asm!("sfence.vma") };
    Ok(())
}
