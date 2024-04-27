//! Loading and execution of the init process

use crate::virtmem;

use crate::virtmem::map_range_alloc;
use allocators::bump_allocator::BumpAllocator;
use elfloader::arch::riscv::RelocationTypes;
use elfloader::{
    ElfLoader, ElfLoaderErr, Flags, LoadableHeaders, RelocationEntry, RelocationType, VAddr,
};
use riscv::mem::mapping::PhysMapping;
use riscv::mem::{EntryFlags, PageTable, PAGESIZE};

/// The location into which the kernel elf file is temporarily copied during boot
static mut TMP_STORE: [u8; 1024 * 1024 * 10] = [0u8; 1024 * 1024 * 10];

/// Copy the elf file located at `ptr` into a temporary, internal location and return a new pointer to it.
///
/// # Safety
/// This function must not be called more than once.
///
/// This function must never be called in a concurrent environment.
pub unsafe fn inline_elf_file(ptr: *const u8, size: usize) -> *const u8 {
    assert!(
        size <= TMP_STORE.len(),
        "kernels elf file is too large for inlined, temporary storage (kerne_size = {size:0x}, capacity = {:0x})",
        TMP_STORE.len()
    );
    log::debug!("moving kernels elf file to temporary, internal location");
    core::intrinsics::copy_nonoverlapping(ptr, TMP_STORE.as_mut_ptr(), size);
    TMP_STORE.as_ptr()
}

/// A simple [`ElfLoader`] implementation that is able to load the kernel binary given only an allocator
pub struct KernelLoader<'alloc, A: BumpAllocator<'static>> {
    pub allocator: &'alloc A,
    pub root_pagetable: &'static mut PageTable,
    pub phys_map: PhysMapping,
}

impl<'alloc, A: BumpAllocator<'static>> KernelLoader<'alloc, A> {
    pub fn new(
        allocator: &'alloc A,
        root_pagetable: &'static mut PageTable,
        phys_map: PhysMapping,
    ) -> Self {
        Self {
            allocator,
            root_pagetable,
            phys_map,
        }
    }

    pub fn load_stack(&mut self, stack_low: u64, stack_high: u64) -> u64 {
        let rw = EntryFlags::Read | EntryFlags::Write;
        log::debug!("loading kernel stack    low = {stack_low:0x}    high = {stack_high:0x}    size = {:0x}", stack_high - stack_low);
        virtmem::map_range_alloc(
            self.allocator,
            self.root_pagetable,
            &self.phys_map,
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
            log::trace!(
                "setting up memory for elf section: base = {:#x} end = {:#x} flags = {}",
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
                &self.phys_map,
                header.virtual_addr(),
                header.mem_size(),
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
                let dst = riscv::mem::mapping::translate(
                    self.root_pagetable,
                    &self.phys_map,
                    base + i as u64 * PAGESIZE as u64,
                );
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
                let paddr = riscv::mem::mapping::translate(
                    self.root_pagetable,
                    &self.phys_map,
                    entry.offset,
                );

                unsafe { *(paddr as *mut u64) = addend }

                Ok(())
            }
            _ => Err(ElfLoaderErr::UnsupportedRelocationEntry),
        }
    }
}
