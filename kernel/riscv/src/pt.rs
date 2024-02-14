//! a module for RISCV Page Tables

use bitflags::bitflags;
use core::fmt::Debug;
use core::fmt::Write;
use core::mem::MaybeUninit;
use static_assertions::{assert_eq_align, assert_eq_size};

/// How large each memory page is
///
/// This effects the alignment and sizes of some data structures that directly interface with the CPU e.g. PageTables
#[deprecated(note = "use definition in mem:: instead")]
pub const PAGESIZE: usize = crate::mem::PAGESIZE;

/// Type definition for a slice of bytes that is exactly one page large
#[deprecated(note = "use definition in mem:: instead")]
pub type MemoryPage = crate::mem::MemoryPage;

#[deprecated(note = "use definition in mem:: instead")]
pub type PageTableEntry = crate::mem::PageTableEntry;

#[deprecated(note = "use definition in mem:: instead")]
pub type PageTable = crate::mem::PageTable;

#[deprecated(note = "use definition in mem:: instead")]
pub type EntryFlags = crate::mem::EntryFlags;

#[deprecated(note = "use definition in mem:: instead")]
pub type EntryInvalidErr = crate::mem::EntryInvalidErr;

const PBITS: usize = 12; // the page offset is 12 bits long
const PBIT_MASK: usize = (1 << PBITS) - 1;

const PPN_BITS: usize = 56;
const PADDR_MASK: usize = (1 << PPN_BITS) - 1;

// For Sv39 and Sv48, each VPN section has 9 bits in length;
const VPN_BITS: usize = 9;
const VPN_MASK: usize = (1 << VPN_BITS) - 1;

fn vpn_segments(vaddr: usize) -> [usize; 3] {
    let vpn = [
        (vaddr >> (PBITS + 0 * VPN_BITS)) & VPN_MASK,
        (vaddr >> (PBITS + 1 * VPN_BITS)) & VPN_MASK,
        (vaddr >> (PBITS + 2 * VPN_BITS)) & VPN_MASK,
        // if Sv48, there is a level of page tables more
        // (vaddr >> (12 + 3 * VPN_BITS)) & VPN_BIT_MASK,
    ];
    vpn
}

#[derive(Debug)]
pub struct MissingPageErr {
    pub level: usize,
    pub target_vaddr: usize,
}

pub fn map_pt(
    trans: impl crate::PhysMapper,
    root: &mut PageTable,
    target_level: usize,
    vaddr: usize,
    pt: *mut PageTable,
) -> Result<(), MissingPageErr> {
    log::debug!("[map_pt] root: {root:p} vaddr: {vaddr:#x} level: {target_level}");
    let vpn = vpn_segments(vaddr);

    let mut v = &mut root.entries[vpn[2]];
    for level in (0..2).rev() {
        if level == target_level {
            unsafe { v.set(trans.mapped_to_phys_mut(pt) as u64, EntryFlags::Valid) };
            return Ok(());
        }
        if !v.is_valid() {
            return Err(MissingPageErr {
                level,
                target_vaddr: vaddr,
            });
        }
        let next_pt_addr = v.get_addr().unwrap();
        let next_pt = unsafe { trans.phys_to_mapped_mut(next_pt_addr as *mut PageTable) };
        v = unsafe { &mut (*next_pt).entries[vpn[level]] };
    }

    panic!("can't set leaf entry as pt");
}

pub fn unmap(trans: impl crate::PhysMapper, root: &mut PageTable, vaddr: usize, paddr: usize) {
    // physical address should be at least page aligned and in PPN range
    assert!(paddr & PBIT_MASK == 0);
    assert!(paddr & !PADDR_MASK == 0);
    // virtual addresses must also be page aligned
    assert!(vaddr & PBIT_MASK == 0, "vaddr is not page-aligned");

    let vpn = vpn_segments(vaddr);

    let mut v = &mut root.entries[vpn[2]];
    for level in (0..2).rev() {
        if !v.is_valid() {
            log::warn!("missing pt when unmapping page");
            return;
        }
        let next_pt_addr = v.get_addr().unwrap();
        let next_pt = unsafe { trans.phys_to_mapped_mut(next_pt_addr as *mut PageTable) };
        v = unsafe { &mut (*next_pt).entries[vpn[level]] };
    }

    if !v.is_valid() {
        log::warn!("tried to unmap invalid page entry");
        return;
    }
    if !v.is_leaf() {
        log::error!("tried to unmap page entry that is not a leaf");
        panic!();
    }
    if v.get_addr().unwrap() != paddr {
        log::warn!("tried to unmap page at vaddr that currently holds another page: entry {:x} paddr: {:x}", v.get_addr().unwrap(), paddr);
        return;
    }
    unsafe {
        v.clear();
    }
}

pub fn map(
    trans: impl crate::PhysMapper,
    root: &mut PageTable,
    vaddr: usize,
    paddr: usize,
    flags: EntryFlags,
) -> Result<(), MissingPageErr> {
    log::debug!("[map] root: {root:p} vaddr: {vaddr:#x} paddr: {paddr:#x} flags: {flags:?}");
    // Make sure that one of Read, Write, or Execute Bits is set.
    // Otherwise, entry is regarded as pointer to next page table level
    assert_eq!(flags.bits() & EntryFlags::all().bits(), flags.bits());
    assert_ne!((flags & EntryFlags::RWX), EntryFlags::empty());

    // physical address should be at least page aligned and in PPN range
    assert!(paddr & PBIT_MASK == 0);
    assert!(paddr & !PADDR_MASK == 0);
    // virtual addresses must also be page aligned
    assert!(vaddr & PBIT_MASK == 0, "vaddr is not page-aligned");

    let vpn = vpn_segments(vaddr);

    let mut v = &mut root.entries[vpn[2]];
    for level in (0..2).rev() {
        if !v.is_valid() {
            return Err(MissingPageErr {
                level,
                target_vaddr: vaddr,
            });
        }
        let next_pt_addr = v.get_addr().unwrap();
        let next_pt = unsafe { trans.phys_to_mapped_mut(next_pt_addr as *mut PageTable) };
        v = unsafe { &mut (*next_pt).entries[vpn[level]] };
    }

    // Now we are ready to point v to our physical address
    assert!(
        !v.is_valid(),
        "the pagetable entry is already mapped. maybe you're trying to map overlapping pages"
    );
    unsafe {
        v.set(paddr as u64, flags | EntryFlags::Valid);
    }
    Ok(())
}

pub fn virt_to_phys(
    trans: impl crate::PhysMapper,
    root: &PageTable,
    vaddr: usize,
) -> Option<usize> {
    let vpn = vpn_segments(vaddr);
    let v = &root.entries[vpn[2]];
    if !v.is_valid() {
        return None;
    }
    if v.is_leaf() {
        panic!("hugepage encountered");
    }
    let pt = unsafe {
        let next_pt_addr = v.get_addr().unwrap();
        let next_pt = trans.phys_to_mapped(next_pt_addr as *const PageTable);
        &*next_pt
    };
    let v = &pt.entries[vpn[1]];
    if !v.is_valid() {
        return None;
    }
    if v.is_leaf() {
        panic!("hugepage encountered");
    }
    let pt = unsafe {
        let next_pt_addr = v.get_addr().unwrap();
        let next_pt = trans.phys_to_mapped(next_pt_addr as *const PageTable);
        &*next_pt
    };
    let v = &pt.entries[vpn[0]];
    if !v.is_valid() {
        return None;
    }
    if !v.is_leaf() {
        panic!("non leaf page where leaf was expected");
    }

    let address = v.get_addr().unwrap();
    Some(address | (vaddr & PBIT_MASK))
}
