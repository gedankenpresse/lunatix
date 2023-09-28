use crate::boundary_tag_alloc::tags::{AllocationMarker, BeginTag, EndTag, TagsBinding};
use crate::{AllocError, AllocInit, Allocator};
use core::alloc::Layout;
use core::cell::RefCell;
use core::marker::PhantomData;

#[derive(Eq, PartialEq)]
pub(super) struct AllocatorState<'mem, Tags: TagsBinding> {
    pub backing_mem: &'mem mut [u8],
    _tags: PhantomData<Tags>,
}

pub(super) struct BlockIterator<'state, 'mem, Tags: TagsBinding> {
    state: &'state AllocatorState<'mem, Tags>,
    i: usize,
}

impl<'state, 'mem, Tags: TagsBinding> Iterator for BlockIterator<'state, 'mem, Tags> {
    type Item = &'state [u8];

    fn next(&mut self) -> Option<Self::Item> {
        if self.i + 1 >= self.state.backing_mem.len() {
            return None;
        }

        let tag = Tags::BeginTag::read_from_chunk(&self.state.backing_mem[self.i..]);
        let block_start = self.i;
        let block_size = Tags::TAGS_SIZE + tag.content_size().into();
        self.i += block_size;
        Some(&self.state.backing_mem[block_start..block_start + block_size])
    }
}

impl<'mem, Tags: TagsBinding> AllocatorState<'mem, Tags> {
    pub fn block_iter<'a>(&'a self) -> BlockIterator<'a, 'mem, Tags> {
        BlockIterator { state: self, i: 0 }
    }
}

/// A general purpose allocator that attaches boundary tags to handed out memory for bookkeeping.
pub(super) struct BoundaryTagAllocator<'mem, Tags: TagsBinding> {
    pub(super) state: RefCell<AllocatorState<'mem, Tags>>,
    _tags: PhantomData<Tags>,
}

impl<'mem, Tags: TagsBinding> BoundaryTagAllocator<'mem, Tags> {
    /// Create a new allocator that allocates from the given backing memory.
    pub fn new(backing_mem: &'mem mut [u8]) -> Self {
        assert!(
            backing_mem.len() <= Tags::BeginTag::MAX_CONTENT_SIZE,
            "backing memory is too large for the allocator to handle"
        );
        assert!(
            backing_mem.len() > Tags::TAGS_SIZE,
            "backing memory is too small to small"
        );

        // write initial tags into the backing memory
        let usable_len = backing_mem.len() - Tags::TAGS_SIZE;
        Tags::BeginTag::new(usable_len, AllocationMarker::Free).write_to_chunk(backing_mem);
        Tags::EndTag::new(usable_len).write_to_chunk(backing_mem);

        Self {
            state: RefCell::new(AllocatorState {
                backing_mem,
                _tags: PhantomData::default(),
            }),
            _tags: PhantomData::default(),
        }
    }
}

impl<'mem, Tags: TagsBinding> Allocator<'mem> for BoundaryTagAllocator<'mem, Tags> {
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
            let mut begin_tag = Tags::BeginTag::read_from_chunk(&state.backing_mem[i..]);
            let block_size =
                Tags::BeginTag::TAG_SIZE + begin_tag.content_size().into() + Tags::EndTag::TAG_SIZE;
            if begin_tag.state() == AllocationMarker::Allocated {
                i += block_size;
                continue;
            }

            // skip blocks that cannot be used because the required Layout does not fit into them
            let unaligned_content_addr =
                (&mut state.backing_mem[i + Tags::BeginTag::TAG_SIZE]) as *mut u8 as usize;
            let aligned_content_addr =
                (unaligned_content_addr + layout.align() - 1) & !(layout.align() - 1);
            let mut begin_padding = aligned_content_addr - unaligned_content_addr;
            let mut full_content_size = layout.size() + begin_padding;
            if (begin_tag.content_size().into()) < full_content_size {
                i += block_size;
                continue;
            }

            // if we have not skipped any blocks, the current block is free and large enough
            let mut free_block = &mut state.backing_mem[i..i + block_size];

            // if the used padding is so large that an additional allocation would fit into it,
            // carve of that section
            if begin_padding > Tags::TAGS_SIZE {
                let (padding_block, remainder) = free_block.split_at_mut(begin_padding);

                // mark the padding block as free but with reduced size
                let padding_block_len = padding_block.len();
                let padding_content_size = padding_block_len - Tags::TAGS_SIZE;
                Tags::BeginTag::new(padding_content_size, AllocationMarker::Free)
                    .write_to_chunk(&mut padding_block[0..]);
                Tags::EndTag::new(padding_content_size).write_to_chunk(
                    &mut padding_block[padding_block_len - Tags::EndTag::TAG_SIZE..],
                );

                // update the the remainder blocks tags to its new size
                let remainder_content_size = remainder.len() - Tags::TAGS_SIZE;
                Tags::BeginTag::new(remainder_content_size, AllocationMarker::Free)
                    .write_to_chunk(remainder);
                Tags::EndTag::new(remainder_content_size).write_to_chunk(remainder);

                // update values that are used for size calculation to the new blocks size
                begin_tag = Tags::BeginTag::read_from_chunk(&remainder[0..]);
                begin_padding = 0;
                full_content_size = layout.size();
                free_block = remainder;
            }

            // slice of a claim for the allocation from the free block
            let (claimed_block, end_padding) =
                if begin_tag.content_size().into() == full_content_size {
                    // the free block an requested allocation match sizes exactly so we can use the found block as-is
                    (free_block, 0)
                } else if begin_tag.content_size().into() > full_content_size + Tags::TAGS_SIZE {
                    // this branch is taken if the free block is large enough to hold the currently requested allocation
                    // as well as another one later.

                    let (claimed_block, remaining_block) =
                        free_block.split_at_mut(full_content_size + Tags::TAGS_SIZE);

                    // add a new end tag to the claimed block
                    Tags::EndTag::new(full_content_size).write_to_chunk(claimed_block);

                    // mark the remainder as still free by writing a new boundary tag at its beginning and updating the end tag
                    let remaining_block_content_size = remaining_block.len() - Tags::TAGS_SIZE;
                    Tags::BeginTag::new(remaining_block_content_size, AllocationMarker::Free)
                        .write_to_chunk(remaining_block);
                    Tags::EndTag::new(remaining_block_content_size).write_to_chunk(remaining_block);

                    (claimed_block, 0)
                } else {
                    // the free block is larger than it needs to but not large enough to hold an additional allocation
                    // so we use the the free block without any modification but need to apply some padding at the end
                    // so that the handed out allocation is the correct size
                    let end_padding = begin_tag.content_size().into() - full_content_size;
                    full_content_size += end_padding;
                    (free_block, end_padding)
                };

            // mark the block as claimed by updating its boundary tag at the beginning
            Tags::BeginTag::new(full_content_size, AllocationMarker::Allocated)
                .write_to_chunk(claimed_block);
            Tags::EndTag::new(full_content_size).write_to_chunk(claimed_block);

            // get the slice from the claimed block that should hold the content
            let allocation = &mut claimed_block[Tags::BeginTag::TAG_SIZE + begin_padding
                ..Tags::BeginTag::TAG_SIZE + full_content_size - end_padding];
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
