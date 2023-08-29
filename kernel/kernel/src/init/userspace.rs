//! Loading and execution of the init process

use crate::caps::{self, CSpaceIface, Capability, VSpaceIface};
use crate::caps::{KernelAlloc, MemoryIface, TaskIface};
use crate::virtmem;
use crate::InitCaps;

use align_data::{include_aligned, Align16};
use allocators::Box;
use derivation_tree::caps::CapabilityIface;
use derivation_tree::tree::DerivationTree;
use elfloader::{
    ElfBinary, ElfLoader, ElfLoaderErr, Flags, LoadableHeaders, RelocationEntry, RelocationType,
    VAddr,
};
use libkernel::mem::ptrs::PhysMutPtr;
use libkernel::mem::EntryFlags;

static INIT_BIN: &[u8] = include_aligned!(
    Align16,
    "../../../../target/riscv64imac-unknown-none-elf/release/init"
);

/// A struct for allocating and mapping (loading) memory so that it can be used for userspace stack
struct StackLoader<'v, 'm> {
    vbase: u64,
    stack_bytes: u64,
    vspace: &'v mut caps::Capability,
    mem: &'m caps::Capability,
}

impl<'a, 'b> StackLoader<'a, 'b> {
    /// Perform the allocate and map operation
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

/// An ElfLoader implementation that loads the elf binary into the configured vspace
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

/// Initialize the derivation tree with necessary init capabilities.
pub fn create_init_caps(
    alloc: &'static KernelAlloc,
    derivation_tree: &DerivationTree<Capability>,
) -> InitCaps<'static, 'static> {
    // create capability objects for userspace code
    log::debug!("creating capabilities for the init task");
    let mut init_caps = InitCaps {
        init_task: Box::new(Capability::empty(), alloc).unwrap(),
    };

    // initializing root memory capability with remaining free space from the kernel allocator#
    log::debug!("creating root memory capability");
    MemoryIface
        .create_init(
            &mut derivation_tree
                .get_root_cursor()
                .unwrap()
                .get_exclusive()
                .unwrap(),
            alloc,
        )
        .unwrap();
    let mut mem_cap = derivation_tree.get_root_cursor().unwrap();
    let mem_cap = mem_cap.get_exclusive().unwrap();

    log::debug!("deriving task capability from root memory capability");
    TaskIface.derive(&mem_cap, &mut init_caps.init_task);
    let mut task_cap = derivation_tree
        .get_node(unsafe { init_caps.init_task.as_raw() }.0)
        .unwrap();
    let mut task_cap = task_cap.get_exclusive().unwrap();
    let mut task_state = task_cap.get_inner_task_mut().unwrap().state.borrow_mut();

    log::debug!("initializing vspace for the init task");
    VSpaceIface.derive(&mem_cap, &mut task_state.vspace);

    log::debug!("initializing cspace for the init task");
    CSpaceIface.derive(&mem_cap, &mut task_state.cspace, 8);

    log::debug!("copying memory, vspace and cspace of the init task into its cspace");
    {
        // copy memory
        let target_slot = unsafe {
            &mut *task_state
                .cspace
                .get_inner_cspace()
                .unwrap()
                .lookup_raw(1)
                .unwrap()
        };
        MemoryIface.copy(&mem_cap, target_slot);
    }
    {
        // copy cspace
        let target_slot = unsafe {
            &mut *task_state
                .cspace
                .get_inner_cspace()
                .unwrap()
                .lookup_raw(2)
                .unwrap()
        };
        CSpaceIface.copy(&task_state.cspace, target_slot);
    }
    {
        // copy vspace
        let target_slot = unsafe {
            &mut *task_state
                .cspace
                .get_inner_cspace()
                .unwrap()
                .lookup_raw(3)
                .unwrap()
        };
        VSpaceIface.copy(&task_state.vspace, target_slot);
    }

    init_caps
}

pub fn load_init_binary(task_cap: &mut Capability, mem_cap: &mut Capability) {
    log::debug!("loading the init binary");
    let mut task_state = task_cap.get_inner_task_mut().unwrap().state.borrow_mut();

    log::debug!("creating a stack for the init binary and mapping it for the init task");
    let stack_start = StackLoader {
        stack_bytes: 0x1000,
        vbase: 0x10_0000_0000,
        mem: mem_cap,
        vspace: &mut task_state.vspace,
    }
    .load()
    .unwrap();

    log::debug!("loading the init binary into its vspace");
    let elf_binary = ElfBinary::new(INIT_BIN).unwrap();
    let mut elf_loader = VSpaceLoader {
        vbase: 0x0,
        mem: &mem_cap,
        vspace: &mut task_state.vspace,
    };
    elf_binary
        .load(&mut elf_loader)
        .expect("Cannot load init binary");
    let init_entry_point = elf_loader.vbase + elf_binary.entry_point();

    // configure the task for the init binary
    task_state.frame.set_stack_start(stack_start as usize);
    task_state.frame.set_entry_point(init_entry_point as usize);
    // this sets the gp
    task_state.frame.general_purpose_regs[3] = init_entry_point as usize + 0x1000;
}
