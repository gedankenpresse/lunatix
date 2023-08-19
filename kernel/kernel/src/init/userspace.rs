//! Loading and execution of the init process

use crate::caps;
use crate::caps::{KernelAlloc, MemoryIface, Tag, TaskIface};
use crate::virtmem;
use crate::InitCaps;

use align_data::{include_aligned, Align16};
use allocators::Arena;
use derivation_tree::caps::CapabilityIface;
use elfloader::{
    ElfBinary, ElfLoader, ElfLoaderErr, Flags, LoadableHeaders, RelocationEntry, RelocationType,
    VAddr,
};
use libkernel::mem::ptrs::PhysMutPtr;
use libkernel::mem::{EntryFlags, MemoryPage};

static INIT_BIN: &[u8] = include_aligned!(
    Align16,
    "../../../../target/riscv64imac-unknown-none-elf/release/init"
);

struct StackLoader<'v, 'm> {
    vbase: u64,
    stack_bytes: u64,
    vspace: &'v mut caps::Capability,
    mem: &'m caps::Capability,
}

impl<'a, 'b> StackLoader<'a, 'b> {
    // returns virtual address of stack start
    fn load(self) -> Result<u64, caps::Error> {
        let vspace = self.vspace;
        let mem = self.mem;
        let rw = EntryFlags::Read | EntryFlags::Write | EntryFlags::UserReadable;
        vspace
            .get_vspace_mut()
            .unwrap()
            .as_ref()
            .map_range(
                mem,
                self.vbase as usize,
                self.stack_bytes as usize,
                rw.bits() as usize,
            )
            .unwrap();
        Ok(self.vbase + self.stack_bytes)
    }
}

struct VSpaceLoader<'v, 'm> {
    vbase: u64,
    vspace: &'v mut caps::Capability,
    mem: &'m caps::Capability,
}

impl<'a, 'r> ElfLoader for VSpaceLoader<'a, 'r> {
    fn allocate(&mut self, load_headers: LoadableHeaders) -> Result<(), ElfLoaderErr> {
        for header in load_headers {
            let virt_start = header.virtual_addr() + self.vbase;
            let virt_end = virt_start + header.mem_size();
            log::debug!(
                "allocate base = {:#x} end = {:#x} flags = {}",
                virt_start,
                virt_end,
                header.flags()
            );

            // maybe this should be done by the VSpace map operation
            let mut bits: EntryFlags = EntryFlags::UserReadable;
            if header.flags().is_execute() {
                bits |= EntryFlags::Execute;
            }
            if header.flags().is_read() {
                bits |= EntryFlags::Read;
            }
            if header.flags().is_write() {
                bits |= EntryFlags::Write;
            }

            self.vspace
                .get_vspace_mut()
                .unwrap()
                .as_ref()
                .map_range(
                    &mut self.mem,
                    virt_start as usize,
                    header.mem_size() as usize,
                    bits.bits() as usize,
                )
                .unwrap();
        }
        Ok(())
    }

    fn load(&mut self, flags: Flags, base: VAddr, region: &[u8]) -> Result<(), ElfLoaderErr> {
        let mut vspaceref = self.vspace.get_vspace_mut().unwrap();
        let vspaceref = vspaceref.as_mut();
        let start = self.vbase + base;
        let end = self.vbase + base + region.len() as u64;
        log::debug!(
            "loading region into = {:#x} -- {:#x}, {}",
            start,
            end,
            flags
        );
        for (offset, byte) in region.iter().enumerate() {
            let addr = start + offset as u64;
            let phys =
                virtmem::virt_to_phys(unsafe { vspaceref.root.as_ref().unwrap() }, addr as usize)
                    .expect("should have been mapped");
            unsafe {
                PhysMutPtr::from(phys as *mut u8)
                    .as_mapped()
                    .raw()
                    .write(*byte)
            }
        }
        Ok(())
    }

    fn relocate(&mut self, entry: RelocationEntry) -> Result<(), ElfLoaderErr> {
        let mut vspaceref = self.vspace.get_vspace_mut().unwrap();
        let vspaceref = vspaceref.as_mut();

        use elfloader::arch::riscv::RelocationTypes;
        use RelocationType::RiscV;
        let addr: *mut u64 = (self.vbase + entry.offset) as *mut u64;
        log::debug!("{:?}", entry.rtype);
        match entry.rtype {
            RiscV(RelocationTypes::R_RISCV_RELATIVE) => {
                // This type requires addend to be present
                let addend = entry
                    .addend
                    .ok_or(ElfLoaderErr::UnsupportedRelocationEntry)?;

                // This is a relative relocation, add the offset (where we put our
                // binary in the vspace) to the addend and we're done.
                log::debug!("R_RELATIV *{:p} = {:#x}", addr, self.vbase + addend);
                // set vspace address through kernel memory mapping
                let phys = virtmem::virt_to_phys(
                    unsafe { vspaceref.root.as_ref().unwrap() },
                    addr as usize,
                )
                .expect("should have been mapped");
                unsafe {
                    PhysMutPtr::from(phys as *mut u64)
                        .as_mapped()
                        .raw()
                        .write(self.vbase + addend)
                }
                Ok(())
            }
            RiscV(RelocationTypes::R_RISCV_64) => {
                log::warn!("R_RISCV_64 not implemented");
                Ok((/* not implemented */))
            }
            _ => Err(ElfLoaderErr::UnsupportedRelocationEntry),
        }
    }
}

/// Initialize [`INIT_CAPS`](crate::INIT_CAPS) with appropriate capabilities
pub fn create_init_caps(alloc: &'static KernelAlloc) {
    // create capability objects for userspace code
    log::debug!("creating capabilities for the init task");
    let mut guard = crate::INIT_CAPS.try_lock().unwrap();

    match &mut *guard {
        InitCaps { mem, init_task } => {
            log::debug!("creating root memory capability");
            MemoryIface.create_init(mem, alloc).unwrap();

            log::debug!("deriving task capability from root memory capability");
            TaskIface.derive(&mem, init_task);

            todo!();

            // let taskstate = unsafe { init_task.get_task_mut().unwrap().state.as_mut().unwrap() };
            // log::debug!("init vspace");
            // mem.derive(taskstate.vspace, |mem| {
            //     caps::VSpaceIface.init(&mut taskstate.vspace, mem)
            // })
            // .unwrap();
            //
            // log::debug!("init cspace");
            // mem.derive(taskstate.cspace, |mem| {
            //     caps::CSpaceIface.init_sz(&taskstate.cspace, mem, 4)
            // })
            // .unwrap();
            // {
            //     let cspace = taskstate.cspace.get_cspace_mut().unwrap();
            //     let memslot = cspace.lookup(1).unwrap();
            //     caps::Memory::copy(mem, memslot).unwrap();
            // }
            // {
            //     let cspace = &taskstate.cspace;
            //     let cref = taskstate.cspace.get_cspace().unwrap();
            //     let target_slot = cref.lookup(2).unwrap();
            //     caps::CSpaceIface.copy(&cspace, &mut target_slot);
            // }
            //
            // log::debug!("setup stack");
            // let stack_start = StackLoader {
            //     stack_bytes: 0x1000,
            //     vbase: 0x10_0000_0000,
            //     mem: &mem,
            //     vspace: &mut taskstate.vspace,
            // }
            // .load()
            // .unwrap();
            //
            // // load elf binary
            // log::debug!("load elf binary");
            // let mut elf_loader = VSpaceLoader {
            //     // choosing arbitrary vbase not supported for relocating data sections
            //     // vbase: 0x5_0000_0000,
            //     vbase: 0x0,
            //     mem: &mem,
            //     vspace: &mut taskstate.vspace,
            // };
            //
            // let binary = ElfBinary::new(INIT_BIN).unwrap();
            // binary.load(&mut elf_loader).expect("Cant load the binary?");
            // let entry_point = binary.entry_point() + elf_loader.vbase;
            //
            // // set stack pointer
            // taskstate.frame.set_stack_start(stack_start as usize);
            //
            // // try setting gp
            // // taskstate.frame.general_purpose_regs[3] = entry_point as usize + 0x1000;
            //
            // // set up program counter to point to userspace code
            // log::debug!("entry point: {:0x}", entry_point);
            // taskstate.frame.set_entry_point(entry_point as usize);
        }
    }
}
