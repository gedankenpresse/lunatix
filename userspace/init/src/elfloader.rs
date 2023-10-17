use core::mem;
use elfloader::{ElfLoader, ElfLoaderErr, Flags, LoadableHeaders, RelocationEntry, VAddr};
use librust::println;
use librust::syscall_abi::identify::CapabilityVariant;
use librust::syscall_abi::CAddr;
use librust::syscall_abi::MapFlags;

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
pub struct LunatixElfLoader<const MAX_NUM_PAGES: usize> {
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

    free_pages: [Option<CAddr>; MAX_NUM_PAGES],
    used_pages: [Option<Mapping>; MAX_NUM_PAGES],
}

impl<const MAX_NUM_PAGES: usize> LunatixElfLoader<MAX_NUM_PAGES> {
    pub fn new(
        mem: CAddr,
        own_vspace: CAddr,
        target_vspace: CAddr,
        caddr_page_start: usize,
        interim_addr: usize,
    ) -> Self {
        let mut free_pages = [None; MAX_NUM_PAGES];
        for i in 0..MAX_NUM_PAGES {
            free_pages[i] = Some(caddr_page_start + i);
        }

        Self {
            mem,
            own_vspace,
            target_vspace,
            free_pages,
            interim_addr,
            used_pages: [None; MAX_NUM_PAGES],
        }
    }

    fn claim_free_page(
        &mut self,
        local_addr: usize,
        target_addr: usize,
        target_flags: MapFlags,
    ) -> Option<&Mapping> {
        let free_page_ref = self.free_pages.iter_mut().find(|i| i.is_some())?;
        let mut tmp = None;
        mem::swap(free_page_ref, &mut tmp);

        let mut tmp = Some(Mapping {
            page: tmp.unwrap(),
            local_addr,
            target_addr,
            flags: target_flags,
        });

        let used_page_ref = self.used_pages.iter_mut().find(|i| i.is_none())?;
        mem::swap(used_page_ref, &mut tmp);
        used_page_ref.as_ref()
    }

    fn find_mapping(&self, target_addr: usize) -> Option<&Mapping> {
        self.used_pages
            .iter()
            .filter_map(|i| i.as_ref())
            .find(|i| i.target_addr <= target_addr && i.target_addr + PAGESIZE > target_addr)
    }

    pub fn remap_to_target_vspace(&mut self) {
        println!("remapping to target vspace");
        for m in self.used_pages {
            let Some(mapping) = m else {
                continue;
            };
            librust::unmap_page(mapping.page).unwrap();
            librust::map_page(
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

impl<const MAX_NUM_PAGES: usize> ElfLoader for LunatixElfLoader<MAX_NUM_PAGES> {
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
                println!(
                    "allocating region {:x?} from elf-offset={:x}",
                    mapping,
                    load_header.offset()
                );
                self.interim_addr += PAGESIZE;
                librust::derive(self.mem, mapping.page, CapabilityVariant::Page, None).unwrap();
                // map page for us so we can load content into it later
                println!("mapping page {} {:x}", mapping.page, mapping.local_addr);
                librust::map_page(
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
            println!("loading content of region {:?}", mapping);
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
