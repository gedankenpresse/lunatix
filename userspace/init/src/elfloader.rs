use core::cmp::min;
use core::mem;
use elfloader::{ElfLoader, ElfLoaderErr, Flags, LoadableHeaders, RelocationEntry, VAddr};
use librust::println;
use librust::syscall_abi::derive_from_mem::DeriveFromMemReturn;
use librust::syscall_abi::identify::CapabilityVariant;
use librust::syscall_abi::map_page::{MapPageFlag, MapPageReturn};
use librust::syscall_abi::CAddr;

const PAGESIZE: usize = 4096;

/// Data for tracking which virtual addresses are mapped where into the local address space and which Page capability
/// is used for it.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
struct Mapping {
    page: CAddr,
    local_addr: usize,
    target_addr: usize,
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

    fn claim_free_page(&mut self, local_addr: usize, target_addr: usize) -> Option<&Mapping> {
        let free_page_ref = self.free_pages.iter_mut().find(|i| i.is_some())?;
        let mut tmp = None;
        mem::swap(free_page_ref, &mut tmp);

        let mut tmp = Some(Mapping {
            page: tmp.unwrap(),
            local_addr,
            target_addr,
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
}

impl<const MAX_NUM_PAGES: usize> ElfLoader for LunatixElfLoader<MAX_NUM_PAGES> {
    fn allocate(&mut self, load_headers: LoadableHeaders) -> Result<(), ElfLoaderErr> {
        for load_header in load_headers {
            for page_offset in (0..load_header.mem_size() as usize).step_by(PAGESIZE) {
                // allocate a page
                let mapping = *self
                    .claim_free_page(
                        self.interim_addr,
                        load_header.virtual_addr() as usize + page_offset,
                    )
                    .unwrap();
                println!(
                    "allocating region {:x?} from elf-offset={:x}",
                    mapping,
                    load_header.offset()
                );
                self.interim_addr += PAGESIZE;
                let alloc_res =
                    librust::derive_from_mem(self.mem, mapping.page, CapabilityVariant::Page, None);
                assert_eq!(alloc_res, DeriveFromMemReturn::Success);

                // map page for us so we can load content into it later
                let map_res = librust::map_page(
                    mapping.page,
                    self.own_vspace,
                    self.mem,
                    mapping.local_addr,
                    MapPageFlag::READ | MapPageFlag::WRITE,
                );
                assert_eq!(map_res, MapPageReturn::Success);

                // map page for the new task with appropriate flags
                let mut flags = MapPageFlag::empty();
                if load_header.flags().is_read() {
                    flags |= MapPageFlag::READ;
                }
                if load_header.flags().is_write() {
                    flags |= MapPageFlag::WRITE;
                }
                if load_header.flags().is_execute() {
                    flags |= MapPageFlag::EXEC;
                }
                let map_res = librust::map_page(
                    mapping.page,
                    self.target_vspace,
                    self.mem,
                    mapping.target_addr,
                    flags,
                );
                assert_eq!(map_res, MapPageReturn::Success);
            }
        }

        Ok(())
    }

    fn load(&mut self, flags: Flags, base: VAddr, region: &[u8]) -> Result<(), ElfLoaderErr> {
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