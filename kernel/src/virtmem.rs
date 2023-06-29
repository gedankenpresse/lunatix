use core::mem::MaybeUninit;
use bitflags::bitflags;
use memory::Arena;

use crate::mem::{Page, PAGESIZE};

#[derive(Copy, Clone)]
pub struct Entry {
    entry: u64,
}

pub struct PageTable {
    entries: [Entry; 512]
}

impl PageTable {
    pub fn empty(alloc: &mut Arena<'static, Page>) -> Option<*mut PageTable> {
       let page = alloc.alloc_one_raw()?;
       unsafe {
        for i in 0..PAGESIZE {
            *page.cast::<u8>().add(i) = 0;
        }
       }
       Some(page.cast::<PageTable>())
    }

    pub fn init(page: *mut MaybeUninit<Page>) -> *mut PageTable {
        unsafe {
            for i in 0..PAGESIZE {
                *page.cast::<u8>().add(i) = 0;
            }
        }
        page.cast::<PageTable>()
    }

    // This doesn't do a deep copy, so it should only be used for global mappings
    pub fn init_copy(page: *mut MaybeUninit<Page>, orig: &PageTable) -> *mut PageTable {
        let root = PageTable::init(page);
        let root_ref = unsafe { root.as_mut().unwrap() };
        for (i, &entry) in orig.entries.iter().enumerate() {
            if entry.is_valid() {
                root_ref.entries[i] = entry;
            }
        }
        return root;
    } 
}


impl PageTable {
    pub fn len() -> usize {
        return 512;
    }
}

bitflags! {
    #[derive(Debug)]
    pub struct EntryBits: u64 {
        const Valid = 1 << 0;
        const Read = 1 << 1;
        const Write = 1 << 2;
        const Execute = 1 << 3;
        const UserReadable = 1 << 4;
        const Global = 1 << 5;
        const Accessed = 1 << 6;
        const Dirty = 1 << 7;

        const RWX = Self::Read.bits() | Self::Write.bits() | Self::Execute.bits();
    }
}


pub enum EntryError {
    EntryInvalid,
    EntryIsPage,
}

impl Entry {
    pub fn is_valid(&self) -> bool {
        self.entry & EntryBits::Valid.bits() != 0
    }

    pub fn is_invalid(&self) -> bool {
        !self.is_valid()
    }
	pub fn is_leaf(&self) -> bool {
		self.entry & EntryBits::RWX.bits() != 0
	}

    pub fn is_pt(&self) -> bool {
        self.entry & EntryBits::RWX.bits() == 0
    }

    pub unsafe fn get_ptr(& self) -> *const PageTable {
        // TODO: Is this correct?
        ((self.entry << 2) & !((1<<12) - 1)) as *mut PageTable
    }

    pub unsafe fn get_ptr_mut(&mut self) -> *mut PageTable {
        // TODO: Is this correct?
        ((self.entry << 2) & !((1<<12) - 1)) as *mut PageTable
    }


    pub unsafe fn get_ptr_usize(& self) -> usize {
        // TODO: Is this correct?
        ((self.entry << 2) & !((1<<12) - 1)) as usize
    }

    pub unsafe fn get_pagetable_mut(&mut self) -> Result<&mut PageTable, EntryError> {
        if self.is_invalid() {
            return Err(EntryError::EntryInvalid);
        }
        if self.is_pt() {
            return Ok(self.get_ptr_mut().as_mut().unwrap());
        }
        return Err(EntryError::EntryIsPage);
    }
}


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
pub enum MapError {
    MappingExists,
    PageTableMissing { level: usize },
}


pub fn map_available(root: &mut PageTable, vaddr: usize, paddr: usize, bits: usize) -> Result<(), MapError> {
    // Make sure that one of Read, Write, or Execute Bits is set.
    // Otherwise, entry is regarded as pointer to next page table level
    assert!(bits & EntryBits::RWX.bits() as usize != 0);
    assert!(bits & !(EntryBits::all().bits() as usize) == 0);

    let vpn = vpn_segments(vaddr);
    let v = &mut root.entries[vpn[2]];
    let level1 = unsafe { v.get_pagetable_mut().map_err(|e| 
        match e {
            EntryError::EntryInvalid => MapError::PageTableMissing { level: 0 },
            EntryError::EntryIsPage => MapError::MappingExists,
        }    
    )?};
    

    Ok(())
}

pub fn map(alloc:  &mut memory::Arena<'static, Page>, root: &mut PageTable, vaddr: usize, paddr: usize, bits: usize) {
    // Make sure that one of Read, Write, or Execute Bits is set.
    // Otherwise, entry is regarded as pointer to next page table level
    assert!(bits & EntryBits::RWX.bits() as usize != 0);
    assert!(bits & !((1 << 10) - 1) == 0);

    // physical address should be at least page aligned and in PPN range
    assert!(paddr & PBIT_MASK == 0);
    assert!(paddr & !PADDR_MASK == 0);

    let vpn = vpn_segments(vaddr);

    // Helper to allocate intermediate page tables
    fn alloc_missing_page(entry: &mut Entry, alloc: &mut Arena<'static, Page>) {
        assert!(entry.is_invalid());

		// Allocate a page
		let page = alloc.alloc_one_raw().expect("could not allocate page").cast::<Page>();
        for i in 0..PAGESIZE {
            unsafe { *page.cast::<u8>().add(i) = 0; }
        }

		entry.entry = (page as u64 >> 2) | EntryBits::Valid.bits();
    }
    // Lookup in top level page table
    let v = &mut root.entries[vpn[2]];
    if !v.is_valid() {
        alloc_missing_page(v, alloc);
	}
    let pt = unsafe { v.get_ptr_mut().as_mut().unwrap() };
    let v = &mut pt.entries[vpn[1]];
    if !v.is_valid() {
        alloc_missing_page(v, alloc);
    }

    // Lookup in lowest level page table
    let pt = unsafe { v.get_ptr_mut().as_mut().unwrap() };
    let v = &mut pt.entries[vpn[0]];


    // Now we are ready to point v to our physical address
    v.entry = ((paddr >> 2) | bits | EntryBits::Valid.bits() as usize) as u64;
}

pub fn virt_to_phys(root: &PageTable, vaddr: usize) -> Option<usize> {
    let vpn = vpn_segments(vaddr);
	let v = &root.entries[vpn[2]];
    if v.is_invalid() {
        return None;
    }
    if v.is_leaf() {
        panic!("hugepage encountered");
    }
    let pt = unsafe { v.get_ptr().as_ref().unwrap() };
    let v = &pt.entries[vpn[1]];
    if v.is_invalid() {
        return None;
    }
    if v.is_leaf() {
        panic!("hugepage encountered");
    }
    let pt = unsafe { v.get_ptr().as_ref().unwrap() };
    let v = &pt.entries[vpn[0]];
    if v.is_invalid() {
        return None;
    }
    if !v.is_leaf() {
        panic!("non leaf page where leaf was expected");
    }

    let address = unsafe { v.get_ptr_usize() };
    return Some(address | (vaddr & PBIT_MASK));
}	


pub fn id_map_range(alloc: &mut Arena<'static, Page>, root: &mut PageTable, start: usize, end: usize, bits: usize) {
    let ptr: *mut Page = (start & !(PAGESIZE - 1)) as *mut Page;
    let endptr: *mut Page = end as *mut Page;
    assert!(ptr <= endptr);
    crate::println!("[id_map] start {:0x} end {:0x}", start, end);
    let mut offset = 0;
    while unsafe { ptr.add(offset) < endptr } {
        let addr = unsafe { ptr.add(offset) } as usize;
        map(alloc, root, addr, addr, bits);
        offset += 1;
    }
}

pub fn map_range_alloc(alloc: &mut Arena<'static, Page>, root: &mut PageTable, virt_base: usize, size: usize, bits: usize) {
    let ptr: *mut Page = (virt_base & !(PAGESIZE - 1)) as *mut Page;
    let mut offset = 0;
    while unsafe { (ptr.add(offset) as usize) < (virt_base + size) } {
        let addr = unsafe { ptr.add(offset) } as usize;
        crate::println!("mapping page {:x}", addr);
        let page_addr = alloc.alloc_one_raw().expect("Could not alloc page").cast::<Page>();
        map(alloc, root, addr, page_addr as usize, bits);
        offset += 1;
    }
}

pub fn create_kernel_page_table(allocator: &mut Arena<'static, Page>, mem_start: usize, mem_length: usize) -> Result<*mut PageTable, ()> {
    let root = PageTable::empty(allocator).unwrap();
    let root_ref = unsafe  { root.as_mut().unwrap() };
    let rwx = EntryBits::RWX.bits() as usize;
    // Map Kernel Memory
    id_map_range(allocator, root_ref, mem_start, mem_start + mem_length, rwx);
    // Map Uart
    id_map_range(allocator, root_ref, 0x1000_0000, 0x1000_0000 + 0x1000, rwx);
    // Map Shutdown
    id_map_range(allocator, root_ref, 0x100_000, 0x100_000 + 0x1000, rwx);
    return Ok(root);
}

pub unsafe fn use_pagetable(root: *mut PageTable) {
    use crate::arch::cpu::*;
 
    // enable MXR (make Executable readable) bit
    // enable SUM (premit Supervisor User Memory access) bit
    unsafe { SStatus::set(SStatusFlags::MXR & SStatusFlags::SUM); }


    let data = Satp::read();
    crate::println!("[use_pagetable] current satp status {:?}", data);

    unsafe { 
        core::arch::asm!("sfence.vma");
    };

    crate::println!("[use_pagetable] enabling {:p}", root);

    // Setup Root Page table in satp register
    unsafe { Satp::write(SatpData { 
        mode: SatpMode::Sv39,
        asid: 1,
        ppn: root as u64 >> 12 
    });}
}