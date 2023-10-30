use alloc::vec::Vec;
use elfloader::{ElfLoader, ElfLoaderErr, Flags, LoadableHeaders, RelocationEntry, VAddr};
use liblunatix::syscall_abi::identify::CapabilityVariant;
use liblunatix::syscall_abi::CAddr;
use liblunatix::syscall_abi::MapFlags;

use crate::static_vec::StaticVec;
use caddr_alloc;

const PAGESIZE: usize = 4096;

/// Data for tracking which virtual addresses are mapped where into the local address space and which Page capability
/// is used for it.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
struct Mapping {
    page: CAddr,
    local_addr: usize,
    target_addr: usize,
    flags: MapFlags,
}

/// An Elfloader implementation that uses lunatix kernel capabilities to allocate pages, map them
/// and then load content into them from the elf binary.
pub struct LunatixElfLoader {
    /// The memory capability from which pages are allocated
    mem: CAddr,
    /// The vspace capability that is mapped to the currently active task.
    /// Content of the elf binary is loaded by mapping pages into this vspace and then storing the elf content
    /// inside it.
    own_vspace: CAddr,
    /// The vspace capability which is used by the task that will execute the elf binary.
    target_vspace: CAddr,
    /// Address at which pages are mapped while content is loaded into them.
    interim_addr: usize,

    used_pages: Vec<Mapping>,
}

impl LunatixElfLoader {
    pub fn new(mem: CAddr, own_vspace: CAddr, target_vspace: CAddr, interim_addr: usize) -> Self {
        Self {
            mem,
            own_vspace,
            target_vspace,
            interim_addr,
            used_pages: Vec::with_capacity(8),
        }
    }

    fn claim_free_page(
        &mut self,
        local_addr: usize,
        target_addr: usize,
        target_flags: MapFlags,
    ) -> Option<&Mapping> {
        let page = caddr_alloc::alloc_caddr();
        let mapping = Mapping {
            page,
            local_addr,
            target_addr,
            flags: target_flags,
        };
        self.used_pages.push(mapping);
        let mapping = &self.used_pages[self.used_pages.len() - 1];
        Some(mapping)
    }

    fn find_mapping(&self, target_addr: usize) -> Option<&Mapping> {
        self.used_pages
            .iter()
            .find(|i| i.target_addr <= target_addr && i.target_addr + PAGESIZE > target_addr)
    }

    pub fn remap_to_target_vspace(&mut self) {
        for mapping in self.used_pages.iter() {
            log::trace!("remapping {mapping:x?} to target vspace");
            liblunatix::unmap_page(mapping.page).unwrap();
            liblunatix::map_page(
                mapping.page,
                self.target_vspace,
                self.mem,
                mapping.target_addr,
                mapping.flags,
            )
            .unwrap();
        }
    }
}

impl ElfLoader for LunatixElfLoader {
    fn allocate(&mut self, load_headers: LoadableHeaders) -> Result<(), ElfLoaderErr> {
        for load_header in load_headers {
            for page_offset in (0..load_header.mem_size() as usize).step_by(PAGESIZE) {
                // calculate mapping flags for target VSPACE
                let mut flags = MapFlags::empty();
                if load_header.flags().is_read() {
                    flags |= MapFlags::READ;
                }
                if load_header.flags().is_write() {
                    flags |= MapFlags::WRITE;
                }
                if load_header.flags().is_execute() {
                    flags |= MapFlags::EXEC;
                }

                // allocate a page
                let mapping = *self
                    .claim_free_page(
                        self.interim_addr,
                        load_header.virtual_addr() as usize + page_offset,
                        flags,
                    )
                    .unwrap();
                log::trace!(
                    "allocating region {:x?} from elf-offset={:x}",
                    mapping,
                    load_header.offset()
                );
                self.interim_addr += PAGESIZE;
                liblunatix::derive(self.mem, mapping.page, CapabilityVariant::Page, None).unwrap();
                // map page for us so we can load content into it later
                log::trace!("mapping page {} {:x}", mapping.page, mapping.local_addr);
                liblunatix::map_page(
                    mapping.page,
                    self.own_vspace,
                    self.mem,
                    mapping.local_addr,
                    MapFlags::READ | MapFlags::WRITE,
                )
                .unwrap();
            }
        }

        Ok(())
    }

    fn load(&mut self, _flags: Flags, base: VAddr, region: &[u8]) -> Result<(), ElfLoaderErr> {
        for (i, chunk) in region.chunks(PAGESIZE).enumerate() {
            let mapping = self.find_mapping(base as usize + i * PAGESIZE).unwrap();
            log::trace!("loading content of region {mapping:x?}");
            unsafe {
                core::intrinsics::copy_nonoverlapping(
                    chunk.as_ptr(),
                    mapping.local_addr as *mut u8,
                    chunk.len(),
                );
            }
        }

        Ok(())
    }

    fn relocate(&mut self, _entry: RelocationEntry) -> Result<(), ElfLoaderErr> {
        unimplemented!("relocation is not implemented by the lunatix elf loader")
    }
}
