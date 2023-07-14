//! Loading and execution of the init process

use crate::caps;
use crate::virtmem;
use crate::InitCaps;

use align_data::{include_aligned, Align16};
use allocators::Arena;
use elfloader::{
    ElfBinary, ElfLoader, ElfLoaderErr, Flags, LoadableHeaders, RelocationEntry, RelocationType,
    VAddr,
};
use libkernel::mem::ptrs::PhysMutPtr;
use libkernel::mem::{EntryFlags, MemoryPage};

static INIT_BIN: &[u8] = include_aligned!(Align16, "../../../target/riscv64imac-unknown-none-elf/release/init");

struct StackLoader<'v, 'm> {
    vbase: u64,
    stack_bytes: u64,
    vspace: &'v mut caps::CNode,
    mem: &'m mut caps::CNode,
}

impl<'a, 'b> StackLoader<'a, 'b> {
    // returns virtual address of stack start
    fn load(self) -> Result<u64, caps::Error> {
        let vspace = self.vspace;
        let mem = self.mem;
        let rw = EntryFlags::Read | EntryFlags::Write | EntryFlags::UserReadable;
        vspace.get_vspace_mut().unwrap().elem.map_range(
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
    vspace: &'v mut caps::CNode,
    mem: &'m mut caps::CNode,
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

            self.vspace.get_vspace_mut().unwrap().elem.map_range(
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
        let vspaceref = self.vspace.get_vspace_mut().unwrap();
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
                virtmem::virt_to_phys(unsafe { vspaceref.elem.root.as_ref().unwrap() }, addr as usize)
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
        let vspaceref = self.vspace.get_vspace_mut().unwrap();

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
                    unsafe { vspaceref.elem.root.as_ref().unwrap() },
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

// Fill INIT_CAPS with appropriate capabilities
pub(crate) fn create_init_caps(alloc: Arena<'static, MemoryPage>) {
    // create capability objects for userspace code
    log::debug!("locking INIT_CAPS");
    let mut guard = crate::INIT_CAPS.try_lock().unwrap();
    guard.mem.set(caps::Memory::create_init(alloc)).unwrap();
    match &mut *guard {
        InitCaps { mem, init_task } => {
            log::debug!("init task");
            caps::Task::init(init_task, &mut mem.cap).unwrap();
            let taskstate = unsafe {
                init_task
                    .cap
                    .get_task_mut()
                    .unwrap()
                    .elem
                    .state
                    .as_mut()
                    .unwrap()
            };
            log::debug!("init vspace");
            caps::VSpace::init(&mut taskstate.vspace, &mut mem.cap).unwrap();

            log::debug!("init cspace");
            caps::CSpace::init_sz(&mut taskstate.cspace, &mut mem.cap, 4).unwrap();
            {
                let cspace = taskstate.cspace.cap.get_cspace_mut().unwrap();
                let memslot = cspace.elem.lookup(1).unwrap();
                caps::Memory::copy(&mut mem.cap, &mut memslot.borrow_mut().cap).unwrap();
            }

            log::debug!("setup stack");
            let stack_start = StackLoader {
                stack_bytes: 0x1000,
                vbase: 0x10_0000_0000,
                mem: &mut mem.cap,
                vspace: &mut taskstate.vspace.cap,
            }
            .load()
            .unwrap();

            // load elf binary
            log::debug!("load elf binary");
            let mut elf_loader = VSpaceLoader {
                // choosing arbitrary vbase not supported for relocating data sections
                // vbase: 0x5_0000_0000,
                vbase: 0x0,
                mem: &mut mem.cap,
                vspace: &mut taskstate.vspace.cap,
            };

            let binary = ElfBinary::new(INIT_BIN).unwrap();
            binary.load(&mut elf_loader).expect("Cant load the binary?");
            let entry_point = binary.entry_point() + elf_loader.vbase;

            // set stack pointer
            taskstate.frame.general_purpose_regs[2] = stack_start as usize;

            // try setting gp
            // taskstate.frame.general_purpose_regs[3] = entry_point as usize + 0x1000;

            // set up program counter to point to userspace code
            taskstate.frame.start_pc = entry_point as usize;
            log::debug!("entry point: {:0x}", taskstate.frame.start_pc);
        }
    }
}
