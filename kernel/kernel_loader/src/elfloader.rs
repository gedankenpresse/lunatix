//! Loading and execution of the init process

use crate::virtmem;

use crate::virtmem::{map_range_alloc, virt_to_phys};
use allocators::bump_allocator::BumpAllocator;
use elfloader::arch::riscv::RelocationTypes;
use elfloader::{
    ElfLoader, ElfLoaderErr, Flags, LoadableHeaders, RelocationEntry, RelocationType, VAddr,
};
use libkernel::mem::{EntryFlags, PageTable};
use riscv::pt::PAGESIZE;

/// A simple [`ElfLoader`] implementation that is able to load the kernel binary given only an allocator
pub struct KernelLoader<'alloc, A: BumpAllocator<'static>> {
    pub allocator: &'alloc A,
    pub root_pagetable: &'static mut PageTable,
}

impl<'alloc, A: BumpAllocator<'static>> KernelLoader<'alloc, A> {
    pub fn new(allocator: &'alloc A, root_pagetable: &'static mut PageTable) -> Self {
        Self {
            allocator,
            root_pagetable,
        }
    }

    pub fn load_stack(&mut self, stack_low: usize, stack_high: usize) -> u64 {
        let rw = EntryFlags::Read | EntryFlags::Write;
        log::debug!("loading stack low: {stack_low:0x} high: {stack_high:0x}");
        virtmem::map_range_alloc(
            self.allocator,
            self.root_pagetable,
            stack_low,
            stack_high - stack_low,
            rw | EntryFlags::Accessed | EntryFlags::Dirty,
        );
        stack_high as u64
    }
}

impl<'alloc, A: BumpAllocator<'static>> ElfLoader for KernelLoader<'alloc, A> {
    fn allocate(&mut self, load_headers: LoadableHeaders) -> Result<(), ElfLoaderErr> {
        for header in load_headers {
            log::debug!(
                "allocating memory for section base = {:#x} end = {:#x} flags = {}",
                header.virtual_addr(),
                header.virtual_addr() + header.mem_size(),
                header.flags(),
            );

            // derive mmu control bits from elf header
            let mut flags: EntryFlags = EntryFlags::empty();
            if header.flags().is_execute() {
                flags |= EntryFlags::Execute;
            }
            if header.flags().is_read() {
                flags |= EntryFlags::Read;
            }
            if header.flags().is_write() {
                flags |= EntryFlags::Write;
            }

            map_range_alloc(
                self.allocator,
                &mut self.root_pagetable,
                header.virtual_addr() as usize,
                header.mem_size() as usize,
                flags | EntryFlags::Accessed | EntryFlags::Dirty,
            );
        }
        Ok(())
    }

    fn load(&mut self, flags: Flags, base: VAddr, region: &[u8]) -> Result<(), ElfLoaderErr> {
        log::debug!(
            "loading elf section data = {:#x} -- {:#x}, {}",
            base,
            base + region.len() as u64,
            flags
        );

        for (i, chunk) in region.chunks(PAGESIZE).enumerate() {
            unsafe {
                let dst = virt_to_phys(self.root_pagetable, (base as usize) + i * PAGESIZE)
                    .expect("Memory mapping was not allocated before being loaded");
                log::trace!(
                    "copying {} bytes from {:p} to {:x}",
                    chunk.len(),
                    chunk.as_ptr(),
                    dst
                );
                core::intrinsics::copy_nonoverlapping::<u8>(
                    chunk.as_ptr(),
                    dst as *mut u8,
                    chunk.len(),
                );
            }
        }

        Ok(())
    }

    fn relocate(&mut self, entry: RelocationEntry) -> Result<(), ElfLoaderErr> {
        match entry.rtype {
            RelocationType::RiscV(RelocationTypes::R_RISCV_RELATIVE) => {
                let addend = entry
                    .addend
                    .ok_or(ElfLoaderErr::UnsupportedRelocationEntry)?;

                // since this is a relative relocation, add the offset to the addend and we're done
                log::debug!("relocating {:?}", entry.rtype);
                let paddr = virt_to_phys(self.root_pagetable, entry.offset as usize)
                    .expect("Memory mapping was not allocated before being relocated");

                unsafe { *(paddr as *mut u64) = addend }

                Ok(())
            }
            _ => Err(ElfLoaderErr::UnsupportedRelocationEntry),
        }
    }
}
