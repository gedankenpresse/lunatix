use crate::{AllocError, AllocInit, Allocator};
use core::alloc::Layout;
use core::cell::RefCell;
use core::mem;

#[derive(Debug, Eq, PartialEq)]
#[repr(u8)]
enum AllocationState {
    Free = 1,
    // this is not zero-indexed so that tags are easier to see while debugging
    Allocated = 2,
}

/// The tag which is placed at the beginning of an allocated memory area.
#[derive(Debug, Eq, PartialEq)]
#[repr(C)]
struct BeginTag {
    block_size: u8,
    state: AllocationState,
}

impl BeginTag {
    fn as_bytes(&self) -> &[u8; 2] {
        unsafe { mem::transmute(self) }
    }

    fn from_bytes(value: &[u8]) -> Self {
        assert_eq!(
            value.len(),
            2,
            "BeginTag can only be reconstructed from 2-byte long slices"
        );
        assert!(
            value[1] == AllocationState::Free as u8 || value[1] == AllocationState::Allocated as u8,
            "value has invalid allocation tag"
        );
        Self {
            block_size: value[0],
            state: unsafe { mem::transmute(value[1]) },
        }
    }
}

/// The tag which is placed at the end of an allocated memory area.
#[derive(Debug, Eq, PartialEq)]
#[repr(C)]
pub(super) struct EndTag {
    block_size: u8,
}

impl EndTag {
    fn as_bytes(&self) -> &u8 {
        unsafe { mem::transmute(self) }
    }

    fn from_bytes(value: &u8) -> &Self {
        unsafe { mem::transmute(value) }
    }
}

#[derive(Eq, PartialEq)]
pub(super) struct AllocatorState<'mem> {
    backing_mem: &'mem mut [u8],
}

struct BlockIterator<'state, 'mem> {
    state: &'state AllocatorState<'mem>,
    i: usize,
}

impl<'state, 'mem> Iterator for BlockIterator<'state, 'mem> {
    type Item = &'state [u8];

    fn next(&mut self) -> Option<Self::Item> {
        if self.i + 1 >= self.state.backing_mem.len() {
            return None;
        }

        let tag = BeginTag::from_bytes(&self.state.backing_mem[self.i..=self.i + 1]);
        let block_start = self.i;
        let block_size =
            mem::size_of::<BeginTag>() + tag.block_size as usize + mem::size_of::<EndTag>();
        self.i += block_size;
        Some(&self.state.backing_mem[block_start..block_start + block_size])
    }
}

impl<'mem> AllocatorState<'mem> {
    fn block_iter<'a>(&'a self) -> BlockIterator<'a, 'mem> {
        BlockIterator { state: self, i: 0 }
    }
}

/// A general purpose allocator that attaches boundary tags to handed out memory for bookkeeping.
pub struct BoundaryTagAllocator<'mem> {
    state: RefCell<AllocatorState<'mem>>,
}

impl<'mem> BoundaryTagAllocator<'mem> {
    /// Create a new allocator that allocates from the given backing memory.
    pub fn new(backing_mem: &'mem mut [u8]) -> Self {
        assert!(
            backing_mem.len() <= u8::MAX as usize,
            "backing memory is too large for the allocator to handle"
        );

        // write initial tags into the backing memory
        let usable_len =
            (backing_mem.len() - mem::size_of::<BeginTag>() - mem::size_of::<EndTag>()) as u8;
        let initial_begin_tag = BeginTag {
            block_size: usable_len,
            state: AllocationState::Free,
        };
        let initial_end_tag = EndTag {
            block_size: usable_len,
        };
        backing_mem[0..2].copy_from_slice(initial_begin_tag.as_bytes());
        backing_mem[backing_mem.len() - 1] = *initial_end_tag.as_bytes();

        Self {
            state: RefCell::new(AllocatorState { backing_mem }),
        }
    }
}

impl<'mem> Allocator<'mem> for BoundaryTagAllocator<'mem> {
    fn allocate(&self, layout: Layout, init: AllocInit) -> Result<&'mem mut [u8], AllocError> {
        assert!(layout.size() > 0, "must allocate at least 1 byte");
        let mut state = self.state.borrow_mut();

        let mut i = 0usize;
        loop {
            // break the loop if we have iterated through all blocks in the backing memory
            if i >= state.backing_mem.len() {
                break;
            }

            // skip blocks that are tagged as Allocated
            let begin_tag = BeginTag::from_bytes(&state.backing_mem[i..=i + 1]);
            let block_size = mem::size_of::<BeginTag>()
                + begin_tag.block_size as usize
                + mem::size_of::<EndTag>();
            if begin_tag.state == AllocationState::Allocated {
                i += block_size;
                continue;
            }

            // skip blocks that cannot be used because the required Layout does not fit into them
            let unaligned_content_addr =
                (&mut state.backing_mem[i + mem::size_of::<BeginTag>()]) as *mut u8 as usize;
            let aligned_content_addr =
                (unaligned_content_addr + layout.align() - 1) & !(layout.align() - 1);
            let begin_padding = aligned_content_addr - unaligned_content_addr;
            let mut full_content_size = layout.size() + begin_padding;
            if (begin_tag.block_size as usize) < full_content_size {
                i += block_size;
                continue;
            }

            // if we have not skipped any blocks, the current block is free and large enough
            let free_block = &mut state.backing_mem[i..i + block_size];

            // slice of a claim for the allocation from the free block
            let (claimed_block, end_padding) = if begin_tag.block_size as usize == full_content_size
            {
                // the free block an requested allocation match sizes exactly so we can use the found block as-is
                (free_block, 0)
            } else if begin_tag.block_size as usize
                > full_content_size + mem::size_of::<BeginTag>() + mem::size_of::<EndTag>()
            {
                // this branch is taken if the free block is large enough to hold the currently requested allocation
                // as well as another one later.

                let (claimed_block, remaining_block) = free_block.split_at_mut(
                    mem::size_of::<BeginTag>() + full_content_size + mem::size_of::<EndTag>(),
                );

                // add a new end tag to the claimed block
                claimed_block[claimed_block.len() - 1] = *EndTag {
                    block_size: full_content_size as u8,
                }
                .as_bytes();

                // mark the remainder as still free by writing a new boundary tag at its beginning and updating the end tag
                let remaining_block_content_size =
                    (remaining_block.len() - mem::size_of::<BeginTag>() - mem::size_of::<EndTag>())
                        as u8;
                remaining_block[0..2].copy_from_slice(
                    BeginTag {
                        state: AllocationState::Free,
                        block_size: remaining_block_content_size,
                    }
                    .as_bytes(),
                );
                remaining_block[remaining_block.len() - 1] = *EndTag {
                    block_size: remaining_block_content_size,
                }
                .as_bytes();

                (claimed_block, 0)
            } else {
                // the free block is larger than it needs to but not large enough to hold an additional allocation
                // so we use the the free block without any modification but need to apply some padding at the end
                // so that the handed out allocation is the correct size
                let end_padding = begin_tag.block_size as usize - full_content_size;
                full_content_size += end_padding;
                (free_block, end_padding)
            };

            // mark the block as claimed by updating its boundary tag at the beginning
            claimed_block[0..2].copy_from_slice(
                BeginTag {
                    state: AllocationState::Allocated,
                    block_size: full_content_size as u8,
                }
                .as_bytes(),
            );
            claimed_block[claimed_block.len() - 1] = *EndTag {
                block_size: full_content_size as u8,
            }
            .as_bytes();

            // get the slice from the claimed block that should hold the content
            let allocation = &mut claimed_block[mem::size_of::<BeginTag>() + begin_padding
                ..mem::size_of::<BeginTag>() + full_content_size - end_padding];
            debug_assert_eq!(allocation.len(), layout.size());

            // initialize the allocation as required
            match init {
                AllocInit::Uninitialized => {}
                AllocInit::Zeroed => {
                    allocation.fill(0);
                }
                AllocInit::Data(data) => {
                    allocation.fill(data);
                }
            }

            // lift lifetime to that of the underlying memory
            // Safety: This is okay because we currently hold the only mutable reference to the backing memory and we have
            // ensured that no aliasing can occur once we release it by writing boundary tags.
            let allocation: &'mem mut [u8] = unsafe { &mut *(allocation as *mut [u8]) };
            return Ok(allocation);
        }

        Err(AllocError::InsufficientMemory)
    }

    unsafe fn deallocate(&self, data_ptr: *mut u8, layout: Layout) {
        todo!()
    }
}

#[cfg(test)]
mod test {
    extern crate alloc;
    extern crate std;

    use super::*;
    use static_assertions::assert_eq_size;
    use std::fmt::{Debug, Formatter};
    use std::vec::Vec;
    use std::{format, println};

    impl Debug for AllocatorState<'_> {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            match f.alternate() {
                false => f
                    .debug_struct("AllocatorState")
                    .field("backing_mem", &self.backing_mem)
                    .finish(),
                true => {
                    let block_reprs: Vec<_> = self
                        .block_iter()
                        .map(|block| {
                            let begin_tag = BeginTag::from_bytes(&block[0..=1]);
                            let end_tag = EndTag::from_bytes(&block[block.len() - 1]);
                            format!(
                                "[<{} {}> ... <{}>]",
                                begin_tag.block_size,
                                match begin_tag.state {
                                    AllocationState::Free => "Free",
                                    AllocationState::Allocated => "Used",
                                },
                                end_tag.block_size
                            )
                        })
                        .collect();

                    f.debug_struct("AllocatorState")
                        .field("backing_mem", &block_reprs.join(" "))
                        .finish()
                }
            }
        }
    }

    assert_eq_size!(BeginTag, [u8; 2]);
    assert_eq_size!(EndTag, u8);

    #[test]
    fn test_initial_tags() {
        let mut mem = [0u8; 8];
        let alloc = BoundaryTagAllocator::new(&mut mem);
        let alloc_state = alloc.state.borrow();
        println!("{:#?}", alloc_state);
        assert_eq!(
            alloc_state.backing_mem,
            [5, AllocationState::Free as u8, 0, 0, 0, 0, 0, 5]
        );
    }

    #[test]
    fn test_alloc_one_u8() {
        let mut mem = [0u8; 8];
        let alloc = BoundaryTagAllocator::new(&mut mem);

        println!("Before Allocation: {:#?}", alloc.state.borrow());
        let block = alloc
            .allocate(Layout::new::<u8>(), AllocInit::Data(0x55))
            .unwrap();
        println!("After Allocation:  {:#?}", alloc.state.borrow());

        assert_eq!(block.len(), mem::size_of::<u8>());
        let alloc_state = alloc.state.borrow();
        assert_eq!(
            alloc_state.backing_mem,
            [
                1,
                AllocationState::Allocated as u8,
                0x55,
                1,
                1,
                AllocationState::Free as u8,
                0,
                1
            ]
        );
    }

    #[test]
    fn test_alloc_multiple_u8() {
        let mut mem = [0u8; 16];
        let alloc = BoundaryTagAllocator::new(&mut mem);

        println!("Initial: {:#?}", alloc.state.borrow());
        let block1 = alloc
            .allocate(Layout::new::<u8>(), AllocInit::Data(0x11))
            .unwrap();
        println!("After allocation 1: {:#?}", alloc.state.borrow());
        let block2 = alloc
            .allocate(Layout::new::<u8>(), AllocInit::Data(0x22))
            .unwrap();
        println!("After allocation 2: {:#?}", alloc.state.borrow());
        let block3 = alloc
            .allocate(Layout::new::<u8>(), AllocInit::Data(0x33))
            .unwrap();
        println!("After allocation 3: {:#?}", alloc.state.borrow());

        assert_eq!(block1.len(), mem::size_of::<u8>());
        assert_eq!(block2.len(), mem::size_of::<u8>());
        assert_eq!(block3.len(), mem::size_of::<u8>());
        let alloc_state = alloc.state.borrow();
        assert_eq!(
            alloc_state.backing_mem,
            [
                1,
                AllocationState::Allocated as u8,
                0x11,
                1,
                1,
                AllocationState::Allocated as u8,
                0x22,
                1,
                1,
                AllocationState::Allocated as u8,
                0x33,
                1,
                1,
                AllocationState::Free as u8,
                0,
                1
            ]
        );
    }

    #[test]
    fn test_alloc_one_u32() {
        let mut mem = [0u8; 16];
        let alloc = BoundaryTagAllocator::new(&mut mem);

        println!("Before Allocation: {:#?}", alloc.state.borrow());
        let block = alloc
            .allocate(Layout::new::<u32>(), AllocInit::Data(0x11))
            .unwrap();
        println!("After Allocation:  {:#?}", alloc.state.borrow());

        assert_eq!(block.len(), mem::size_of::<u32>());
        let alloc_state = alloc.state.borrow();
        assert_eq!(
            alloc_state.backing_mem,
            [
                6,
                AllocationState::Allocated as u8,
                0,
                0,
                0x11,
                0x11,
                0x11,
                0x11,
                6,
                4,
                AllocationState::Free as u8,
                0,
                0,
                0,
                0,
                4,
            ]
        );
    }

    #[test]
    fn test_alloc_last_block() {
        let mut mem = [0u8; 4];
        let alloc = BoundaryTagAllocator::new(&mut mem);
        println!("Before Allocation: {:#?}", alloc.state.borrow());
        let block = alloc
            .allocate(Layout::new::<u8>(), AllocInit::Data(0x11))
            .unwrap();
        println!("After Allocation:  {:#?}", alloc.state.borrow());

        let alloc_state = alloc.state.borrow();
        assert_eq!(
            alloc_state.backing_mem,
            [1, AllocationState::Allocated as u8, 0x11, 1]
        );
    }

    #[test]
    fn test_alloc_one_with_alignment() {
        let mut mem = [0u8; 16];
        let alloc = BoundaryTagAllocator::new(&mut mem);

        println!("Before Allocation: {:#?}", alloc.state.borrow());
        const ALIGNMENT: usize = 4;
        let block = alloc
            .allocate(
                Layout::from_size_align(1, ALIGNMENT).unwrap(),
                AllocInit::Data(0x11),
            )
            .unwrap();
        println!("After Allocation:  {:#?}", alloc.state.borrow());

        assert_eq!((block.as_ptr() as usize) % ALIGNMENT, 0);
        assert_eq!(block.len(), 1);
    }

    #[test]
    fn test_not_enough_mem_for_two_allocs_but_more_than_enough_for_one() {
        let mut mem = [0u8; 5];
        let alloc = BoundaryTagAllocator::new(&mut mem);

        println!("Before Allocation: {:#?}", alloc.state.borrow());
        let block = alloc
            .allocate(Layout::new::<u8>(), AllocInit::Data(0x11))
            .unwrap();
        println!("After Allocation:  {:#?}", alloc.state.borrow());

        assert_eq!(block.len(), 1);
        let alloc_state = alloc.state.borrow();
        assert_eq!(
            alloc_state.backing_mem,
            [2, AllocationState::Allocated as u8, 0x11, 0, 2]
        );
    }
}
