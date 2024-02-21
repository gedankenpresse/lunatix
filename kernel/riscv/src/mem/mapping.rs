//! Manipulation of and interaction with the virtual address mapping configuration

use crate::mem::paddr::PAddr;
use crate::mem::vaddr::VAddr;
use crate::mem::{paddr, vaddr, EntryFlags, PageTable, PageTableEntry};
use allocators::{AllocInit, Allocator};
use core::alloc::Layout;
use core::mem::MaybeUninit;

/// The type (and therefore size) of a mapped page
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum PageType {
    /// Standard `4KiB` (`4096` bytes) page
    Page,
    /// `2MiB` mega page
    MegaPage,
    /// `1GiB` giga page
    GigaPage,
}

impl PageType {
    /// Get the required alignment for this page type
    pub const fn required_alignment(&self) -> u64 {
        self.size()
    }

    // Get the number of bytes that can be stored in a page of this type
    pub const fn size(&self) -> u64 {
        match self {
            PageType::Page => 1 << paddr::PAGE_OFFSET_BITS,
            PageType::MegaPage => 1 << (paddr::PAGE_OFFSET_BITS + paddr::PPN0_BITS),
            PageType::GigaPage => {
                1 << (paddr::PAGE_OFFSET_BITS + paddr::PPN0_BITS + paddr::PPN1_BITS)
            }
        }
    }
}

impl Default for PageType {
    fn default() -> Self {
        PageType::Page
    }
}

/// Description of an area in accessible memory from which the physical memory is loadable
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct PhysMapping {
    pub start: u64,
    pub size: u64,
}

impl PhysMapping {
    /// Return the mapping which describes addresses being identity-mapped.
    /// That is, physical addresses can be loaded from their value directly and do not need to be
    /// mapped.
    pub const fn identity() -> Self {
        Self {
            start: 0,
            size: u64::MAX,
        }
    }

    /// Create a new instance describing physical addresses being available from `start` and is `size` bytes large.
    ///
    /// This means that e.g. the physical address `0x0` is loadable from `start` and the mapping is
    /// only valid for the next `size` bytes after it.
    pub const fn new(start: u64, size: u64) -> Self {
        Self { start, size }
    }

    /// Resolve the given physical address into its loadable mapped variant.
    ///
    /// This method is intended to be used when the input is an address that is understood by the memory management
    /// unit while the output is an address that is loadable by the CPU right now.
    pub const fn map(&self, addr: PAddr) -> u64 {
        assert!(addr < self.size);
        self.start + addr
    }

    /// Reverse-resolve the given mapped address to its actual hardware address.
    ///
    /// This method is intended to be used when the input is an address that is loadable by the CPU right now while
    /// the output is one that is understood by the memory management unit.
    pub const fn rev_map(&self, addr: PAddr) -> u64 {
        assert!(addr < self.start + self.size);
        addr - self.start
    }
}

/// Map the given `vaddr` to point to the given `paddr` with the given `flags`.
///
/// The mapping is set up in the given `root` PageTable while all required intermediate PageTables are automatically
/// allocated.
///
/// In the context of mapped physical memory, `paddr` is assumed to be an address that is loadable by the CPU right
/// now which might be a mapped address.
/// It is automatically translated to the real hardware address using `phys_map`.
///
/// This function panics if an existing mapping would be overridden.
pub fn map<'a>(
    alloc: &impl Allocator<'a>,
    root_pagetable: &mut PageTable,
    phys_map: &PhysMapping,
    vaddr: VAddr,
    paddr: PAddr,
    flags: EntryFlags,
    page_type: PageType,
) {
    log::trace!(
        "configuring address translation mapping {vaddr:#x} -> {paddr:#x} (flags={flags:?}) in page table {root_pagetable:p}"
    );

    // check some preconditions
    assert!(
        flags.intersects(EntryFlags::RWX),
        "an address mapping must set either Read, Write or Execute bits"
    );
    assert_eq!(
        paddr & (page_type.required_alignment() - 1),
        0,
        "cannot use non page-aligned paddr {paddr:#x} as the target of {page_type:?} virtual address mapping"
    );
    assert_eq!(
        paddr & paddr::PADDR_MASK,
        paddr,
        "paddr {paddr:#x} > {:#x} is not supported in Sv39 virtual addressing mode",
        paddr::PADDR_MASK,
    );
    assert_eq!(
        vaddr & (page_type.required_alignment() - 1),
        0,
        "cannot use non page-aligned vaddr {vaddr:#x} as the source of {page_type:?} virtual address mapping"
    );

    let vpn_segments = vaddr::vpn_segments(vaddr);
    match page_type {
        PageType::GigaPage => {
            make_leaf_mapping(
                root_pagetable,
                vpn_segments[2] as usize,
                paddr,
                flags,
                phys_map,
            );
        }
        PageType::MegaPage => {
            let through_table =
                make_through_mapping(root_pagetable, vpn_segments[2] as usize, phys_map, alloc);
            make_leaf_mapping(
                through_table,
                vpn_segments[1] as usize,
                paddr,
                flags,
                phys_map,
            );
        }
        PageType::Page => {
            let through_table =
                make_through_mapping(root_pagetable, vpn_segments[2] as usize, phys_map, alloc);
            let through_table =
                make_through_mapping(through_table, vpn_segments[1] as usize, phys_map, alloc);
            make_leaf_mapping(
                through_table,
                vpn_segments[0] as usize,
                paddr,
                flags,
                phys_map,
            );
        }
    }

    fn make_through_mapping<'a, 'b>(
        table: &mut PageTable,
        entry_no: usize,
        phys_map: &PhysMapping,
        alloc: &impl Allocator<'a>,
    ) -> &'b mut PageTable {
        let entry = &mut table.entries[entry_no];
        match entry.get_addr() {
            Ok(addr) => {
                assert!(!entry.is_leaf());
                let addr = phys_map.map(addr);
                unsafe { (addr as *mut PageTable).as_mut().unwrap() }
            }
            Err(_) => {
                log::trace!("mapping requires new intermediate page table");
                let through_table = alloc
                    .allocate(Layout::new::<PageTable>(), AllocInit::Uninitialized)
                    .expect("Could not allocate space for intermediate page table")
                    .as_mut_ptr()
                    .cast::<MaybeUninit<PageTable>>();
                let through_table = PageTable::init(through_table);
                unsafe { entry.set(phys_map.rev_map(through_table as PAddr), EntryFlags::Valid) };
                unsafe { through_table.as_mut().unwrap() }
            }
        }
    }

    fn make_leaf_mapping(
        table: &mut PageTable,
        entry_no: usize,
        to: PAddr,
        flags: EntryFlags,
        phys_map: &PhysMapping,
    ) {
        let entry = &mut table.entries[entry_no];
        assert!(
            !entry.is_valid(),
            "refusing to override existing address mapping"
        );
        unsafe {
            entry.set(
                phys_map.rev_map(to),
                flags | EntryFlags::Dirty | EntryFlags::Accessed,
            )
        }
    }
}

/// Translate the given `vaddr` by walking the hierarchy of pagetables in software.
///
/// The mapping is translated starting from the given root pagetable.
/// All *intermediate* addresses read from page tables are passed through `phys_map` to make them loadable by the CPU.
/// The return value is notable *not* automatically passed through `phys_map`.
///
/// This function panics if an invalid (disabled) entry is encountered.
pub fn translate(root_pagetable: &PageTable, phys_map: &PhysMapping, vaddr: VAddr) -> PAddr {
    // TODO Improve error handling by not panicking
    let vpn = vaddr::vpn_segments(vaddr);
    let page_offset: u64 = vaddr & vaddr::PAGE_OFFSET_MASK;

    // root to 2nd level page table
    let entry = &root_pagetable.entries[vpn[2] as usize];
    assert!(entry.is_valid());
    let through_table = match entry.is_leaf() {
        true => {
            let addr = entry.get_addr().unwrap();
            assert_eq!(
                addr & (paddr::PPN1_MASK | paddr::PPN0_MASK | paddr::PAGE_OFFSET_MASK),
                0
            );
            return addr
                | (vpn[1] << paddr::PPN1_OFFSET)
                | (vpn[0] << paddr::PPN0_OFFSET)
                | page_offset;
        }
        false => {
            let addr = phys_map.map(entry.get_addr().unwrap());
            let pt_ptr = addr as *const PageTable;
            unsafe { pt_ptr.as_ref().unwrap() }
        }
    };

    // 2dn to 3rd level page table
    let entry = &through_table.entries[vpn[1] as usize];
    assert!(entry.is_valid());
    let through_table = match entry.is_leaf() {
        true => {
            let addr = entry.get_addr().unwrap();
            assert_eq!(addr & (paddr::PPN0_MASK | paddr::PAGE_OFFSET_MASK), 0);
            return addr | (vpn[0] << paddr::PPN0_OFFSET) | page_offset;
        }
        false => {
            let addr = phys_map.map(entry.get_addr().unwrap());
            let pt_ptr = addr as *const PageTable;
            unsafe { pt_ptr.as_ref().unwrap() }
        }
    };

    // 3rd page table to final entry
    let entry = &through_table.entries[vpn[0] as usize];
    assert!(entry.is_valid());
    assert!(entry.is_leaf());
    entry.get_addr().unwrap() | page_offset
}
