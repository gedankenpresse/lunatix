//! Loading and execution of the init process

use crate::caps;
use crate::mem;
use crate::virtmem;
use crate::InitCaps;

use elfloader::{
    ElfBinary, ElfLoader, ElfLoaderErr, Flags, LoadableHeaders, RelocationEntry, RelocationType,
    VAddr,
};

const INIT_BIN: &[u8] = include_bytes!("../../userspace/init_main");

struct StackLoader<'a, 'b> {
    vbase: u64,
    stack_bytes: u64,
    vspace: &'a mut caps::Cap<caps::VSpace>,
    mem: &'b mut caps::Cap<caps::Memory>,
}

impl<'a, 'b> StackLoader<'a, 'b> {
    // returns virtual address of stack start
    fn load(self) -> Result<u64, caps::Error> {
        let vspace = self.vspace;
        let mem = self.mem;
        let rw =
            virtmem::EntryBits::Read | virtmem::EntryBits::Write | virtmem::EntryBits::UserReadable;
        vspace
            .map_range(
                &mut mem.content,
                self.vbase as usize,
                self.stack_bytes as usize,
                rw.bits() as usize,
            )
            .unwrap();
        Ok(self.vbase + self.stack_bytes)
    }
}

struct VSpaceLoader<'m, 'v> {
    vbase: u64,
    mem: &'m mut caps::Cap<caps::Memory>,
    vspace: &'v mut caps::Cap<caps::VSpace>,
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
            let mut bits: virtmem::EntryBits = virtmem::EntryBits::UserReadable;
            if header.flags().is_execute() {
                bits |= virtmem::EntryBits::Execute;
            }
            if header.flags().is_read() {
                bits |= virtmem::EntryBits::Read;
            }
            if header.flags().is_write() {
                bits |= virtmem::EntryBits::Write;
            }

            self.vspace
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
                virtmem::virt_to_phys(unsafe { self.vspace.root.as_ref().unwrap() }, addr as usize)
                    .expect("should have been mapped");
            unsafe {
                *(phys as *mut u8) = *byte;
            }
        }
        Ok(())
    }

    fn relocate(&mut self, entry: RelocationEntry) -> Result<(), ElfLoaderErr> {
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
                    unsafe { self.vspace.root.as_ref().unwrap() },
                    addr as usize,
                )
                .expect("should have been mapped");
                unsafe {
                    *(phys as *mut u64) = self.vbase + addend;
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

// Fill INIT_CAPS with appropriate capabilities
pub(crate) fn create_init_caps(alloc: memory::Arena<'static, mem::Page>) {
    // create capability objects for userspace code
    let mut guard = crate::INIT_CAPS.try_lock().unwrap();
    guard
        .mem
        .set(caps::Cap::from_content(caps::Memory { inner: alloc }))
        .unwrap();
    match &mut *guard {
        InitCaps { mem, init_task } => {
            caps::Task::init(init_task, mem.cap.get_memory_mut().unwrap()).unwrap();
            let mem_cap = mem.cap.get_memory_mut().unwrap();
            let taskstate = unsafe {
                init_task
                    .cap
                    .get_task_mut()
                    .unwrap()
                    .state
                    .as_mut()
                    .unwrap()
            };
            taskstate
                .vspace
                .set(caps::VSpace::init(mem_cap).unwrap())
                .unwrap();
            taskstate
                .cspace
                .set(caps::CSpace::init_sz(mem_cap, 4).unwrap())
                .unwrap();

            // setup stak
            let stack_start = StackLoader {
                mem: mem_cap,
                stack_bytes: 0x1000,
                vbase: 0x10_0000_0000,
                vspace: taskstate.vspace.cap.get_vspace_mut().unwrap(),
            }
            .load()
            .unwrap();

            // load elf binary
            let mut elf_loader = VSpaceLoader {
                // choosing arbitrary vbase not supported for relocating data sections
                // vbase: 0x5_0000_0000,
                vbase: 0x0,
                mem: mem_cap,
                vspace: taskstate.vspace.cap.get_vspace_mut().unwrap(),
            };
            let binary = ElfBinary::new(INIT_BIN).unwrap();
            binary.load(&mut elf_loader).expect("Cant load the binary?");
            let entry_point = binary.entry_point() + elf_loader.vbase;

            // set stack pointer
            taskstate.frame.general_purpose_regs[2] = stack_start as usize;

            // try setting gp
            taskstate.frame.general_purpose_regs[3] = entry_point as usize + 0x1000;

            // set up program counter to point to userspace code
            taskstate.frame.start_pc = entry_point as usize;
            log::debug!("entry point: {:0x}", taskstate.frame.start_pc);
        }
    }
}
