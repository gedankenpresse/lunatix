use crate::boundary_tag_alloc::tags::{AllocationMarker, BeginTag, EndTag, TagsBinding};
use crate::{AllocError, AllocInit, Allocator};
use core::alloc::Layout;
use core::marker::PhantomData;
use ksync::SpinLock;

type Chunk<T> = (
    <T as TagsBinding>::BeginTag,
    *mut u8,
    <T as TagsBinding>::EndTag,
);

/// The internal state of the allocator which contains the backing memory.
///
/// ## Wordings
///
/// In the implementation, some words are used with specific meaning:
/// - **Chunk**:
///     A part of the memory which the allocator manages.
///     Each chunk consists of the parts `begin-tag, content, end-tag`.
///     The padding parts are optional and only necessary in certain edge cases.
/// - **Content**:
///     A part of the allocators memory that is handed out to to users.
/// - **Padding**:
///     A part of the allocators memory that serves no use.
///     It sits between the end-tag and handed out content in certain situations but is neither handed
///     out to the user as content nor otherwise used by the allocator.
/// - **Tag**:
///     A tag stores metadata about the chunk it is contained in and fulfills a bookkeeping role.
///     They come in the two flavors `begin-tag` which lies at the beginning of a chunk and `end-tag` which is located
///     at the end of one.
///
/// # Memory Layout
///
/// The boundary tagged allocator is called that because it writes tags into the beginning and end of chunks of
/// memory (the chunks boundaries) to track their allocation state and size.
///
/// This tagging however is not trivial when handling allocation layouts that require pointer alignment because padding
/// needs to be added which creates a space between a chunks tags and content.
/// This space makes it difficult to handle deallocations because in a naive implementation the allocator cannot
/// know a chunks tag positions only from the content pointer.
/// Because of this, the allocator only applies padding between content and end-tag.
///
/// ## Layout Examples
///
/// In the following sections, the allocation and deallocation strategy is explained in detail using a schematic
/// drawing of the underlying memory.
/// Each box represents a chunk of memory starting with a tag followed by content followed by optional padding and
/// finalized by another tag.
///
/// These examples always use 128 bytes as an example underlying storage size and U8 based tags.
/// It also assumes that the underlying memory is fully aligned.
///
/// ## Initial Memory
///
/// When creating a new allocator, all the backing memory belongs to one large chunk that is marked as free.
///
/// ```text
/// ┌───────────────────────────────────────┐
/// │ "125", free, 125 content-bytes, "125" │
/// └───────────────────────────────────────┘
///  ^                                     ^
///  └─────── 128 bytes total length ──────┘
/// ```
///
/// ## Allocate single bytes
///
/// Assuming the allocator is initially empty, the allocator performs the following steps to allocate a single unaligned
/// byte:
///
/// 1. Search for the first free chunk that has at least 1 byte of content-size.
///
///    Since no allocations have been performed yet, it finds the 125 byte large chunk that spans the whole memory.
///
/// 2. Cut of as much space as needed for the requested 1 byte allocation by splitting the chunk in two, one small chunk
///    for the allocation followed by another large chunk containing the remaining memory.
///
///    This is implemented by updating the initially found chunks start and end tag to new values
///    and then inserting a new end-tag and a new start-tag at the splitting point.
///
///    ```text
///     ┌─────────────────────────────────┐ ┌───────────────────────────────────────┐
///     │ "1", free, 1 content-bytes, "1" │ │ "121", free, 121 content-bytes, "121" │
///     └─────────────────────────────────┘ └───────────────────────────────────────┘
///     ```
///
/// 3. At last, the small chunks begin-tag is updated to indicate that this block is now used by an allocation and a
///    pointer to the content-bytes is returned to the caller.
///
///    ```text
///     ┌─────────────────────────────────┐ ┌───────────────────────────────────────┐
///     │ "1", used, 1 content-bytes, "1" │ │ "121", free, 121 content-bytes, "121" │
///     └─────────────────────────────────┘ └───────────────────────────────────────┘
///     ```
///
/// ## Allocate layouts with small alignments
///
/// When the requested layout of an allocation contains alignment and there is no free block whose content fulfills
/// the requested alignment (a rare circumstance), the allocator must add padding so that the handed out content
/// is aligned.
///
/// This padding however cannot be applied between begin-tag and content because it is required during deallocation
/// that a begin-tag immediately precedes a the content.
/// The only solution is to add padding between content and end-tag which requires patching the preceding chunk
/// to increase its lengths.
///
/// Imagine the following state before an allocation is requested:
///
/// ```text
/// ┌─────────────────────────────────┐ ┌───────────────────────────────────────┐
/// │ "2", used, 2 content-bytes, "2" │ │ "121", free, 121 content-bytes, "121" │
/// └─────────────────────────────────┘ └───────────────────────────────────────┘
///  ^                               ^   ^      ^      ^
///  └──── 5 bytes total length ─────┘   5      6      7
/// ```
///
/// Now an allocation request reaches the allocator that **requests 1 byte content with 8 bytes alignment**.
/// As can be seen in the sketch above, the free chunk does not satisfy the requested 8 byte alignment since its
/// content section starts at address `7`.
/// The allocator must therefore add 1 padding byte and because it cannot be added between begin-tag and content,
/// the preceding chunk must be patched so that padding is added there.
/// After adding the padding, the backing memory looks like this:
///
/// ```text
/// ┌─────────────────────────────────┐ ┌───────────────────────────────────────┐
/// │ "3", used, 3 content-bytes, "3" │ │ "121", free, 121 content-bytes, "121" │
/// └─────────────────────────────────┘ └───────────────────────────────────────┘
///  ^                               ^   ^      ^      ^
///  └──── 6 bytes total length ─────┘   6      7      8
/// ```
///
/// Note that this is safe to do because the already existing allocation is not moved nor shrunken and the tags
/// are never exposed to callers so they don't notice that their available memory has technically just grown.
///
/// After having effectively padded the free chunk so that its content fulfills the requested alignment, the free
/// chunk can be handled as described in the previous example.
///
///
/// ## Allocate layouts with large alignments
///
/// In the previous example, only a small amount of shifting was needed for the free chunk to reach its requested
/// alignment.
/// This is however not always the case, for example when allocating whole pages with 4096 byte alignment, the required
/// shift may be very large.
/// If the previously discussed method of padding the previous chunk were used in this case, many bytes would go to
/// waste which would make the allocator very memory inefficient.
///
/// Let's assume the following memory state before an allocation is requested (total memory size = 200 bytes):
///
/// ```text
/// ┌─────────────────────────────────┐ ┌───────────────────────────────────────┐
/// │ "2", used, 2 content-bytes, "2" │ │ "192", free, 192 content-bytes, "192" │
/// └─────────────────────────────────┘ └───────────────────────────────────────┘
///  ^                               ^   ^      ^      ^
///  └──── 5 bytes total length ─────┘   5      6      7
/// ```
///
/// Now, if the user **requests 1 byte content with 128 byte alignment**, the free block must be shifted to the right
/// by 121 bytes so that the content portion is at address 128.
///
/// The allocator now splits the free chunk into two so that the tail portion of the split has the required content
/// alignment.
///
/// ```text
/// ┌─────────────────────────────────┐ ┌───────────────────────────────────────┐ ┌────────────────────────────────────┐
/// │ "2", used, 2 content-bytes, "2" │ │ "117", free, 117 content-bytes, "117" │ │ "62", free, 62 content-bytes, "62" │
/// └─────────────────────────────────┘ └───────────────────────────────────────┘ └────────────────────────────────────┘
///  ^                               ^   ^                                     ^   ^                                  ^
///  └──── 5 bytes total length ─────┘   └─────── 120 bytes total length ──────┘   └────── 75 bytes total length ─────┘
///                                                                                 ^     ^     ^
///                                                                                126   127   128
/// ```
///
/// After having split the free chunk so that the tails content fulfills the requested alignment, it can be handled
/// as described in the first example.
///
#[derive(Eq, PartialEq)]
pub(super) struct AllocatorState<'mem, Tags: TagsBinding> {
    pub backing_mem: &'mem mut [u8],
    _tags: PhantomData<Tags>,
}

#[cfg(test)]
pub(super) struct BlockIterator<'state, 'mem, Tags: TagsBinding> {
    state: &'state AllocatorState<'mem, Tags>,
    i: usize,
}

#[cfg(test)]
impl<'state, 'mem, Tags: TagsBinding> Iterator for BlockIterator<'state, 'mem, Tags> {
    type Item = &'state [u8];

    fn next(&mut self) -> Option<Self::Item> {
        if self.i + 1 >= self.state.backing_mem.len() {
            return None;
        }

        let tag = Tags::BeginTag::read_from_chunk(&self.state.backing_mem[self.i..]);
        let block_start = self.i;
        let block_size = Tags::TAGS_SIZE + tag.content_size();
        self.i += block_size;
        Some(&self.state.backing_mem[block_start..block_start + block_size])
    }
}

impl<'mem, Tags: TagsBinding> AllocatorState<'mem, Tags> {
    pub(crate) fn new(backing_mem: &'mem mut [u8]) -> Self {
        Self {
            backing_mem,
            _tags: Default::default(),
        }
    }

    #[cfg(test)]
    pub fn block_iter<'a>(&'a self) -> BlockIterator<'a, 'mem, Tags> {
        BlockIterator { state: self, i: 0 }
    }

    /// Get the first chunk of the backing memory
    pub(crate) fn get_first_chunk(&mut self) -> Chunk<Tags> {
        let begin_tag = Tags::BeginTag::read_from_chunk(self.backing_mem);
        let chunk = &mut self.backing_mem[..begin_tag.content_size() + Tags::TAGS_SIZE];
        let end_tag = Tags::EndTag::read_from_chunk(chunk);
        let content = unsafe { chunk.as_mut_ptr().add(Tags::BeginTag::TAG_SIZE) };

        (begin_tag, content, end_tag)
    }

    /// Get the whole chunk from a pointer to the begin-tag
    ///
    /// Returns either the chunk as `Ok` or `Err(())` if the passed pointer is not valid.
    pub(crate) fn get_chunk_from_begin(
        &mut self,
        begin_tag: *mut Tags::BeginTag,
    ) -> Result<Chunk<Tags>, ()> {
        let mem_start_addr = self.backing_mem.as_mut_ptr() as usize;
        let mem_end_addr = mem_start_addr + self.backing_mem.len();
        let begin_tag_addr = begin_tag as usize;

        if begin_tag_addr < mem_start_addr || begin_tag_addr >= mem_end_addr - Tags::TAGS_SIZE {
            return Err(());
        }

        let chunk = &mut self.backing_mem[begin_tag_addr - mem_start_addr..];
        let begin_tag = Tags::BeginTag::read_from_chunk(chunk);
        let chunk = &mut chunk[..Tags::TAGS_SIZE + begin_tag.content_size()];
        let end_tag = Tags::EndTag::read_from_chunk(chunk);
        let content = unsafe { chunk.as_mut_ptr().add(Tags::BeginTag::TAG_SIZE) };

        assert_eq!(begin_tag.content_size(), end_tag.content_size().into());

        Ok((begin_tag, content, end_tag))
    }

    /// Get the whole chunk from a content pointer
    ///
    /// Returns either the chunk as `Ok` or `Err(())` if the passed pointer is not valid.
    pub(crate) fn get_chunk_from_content(&mut self, content: *mut u8) -> Result<Chunk<Tags>, ()> {
        let content_addr = content as usize;
        let begin_tag_addr = content_addr - Tags::BeginTag::TAG_SIZE;
        self.get_chunk_from_begin(begin_tag_addr as *mut Tags::BeginTag)
    }

    /// Get the whole from from a pointer to the end tag.
    ///
    /// Returns either the chunk as `Ok` or `Err(())` if the passed pointer is not valid.
    pub(crate) fn get_chunk_from_end(
        &mut self,
        end_tag: *mut Tags::EndTag,
    ) -> Result<Chunk<Tags>, ()> {
        let mem_start_addr = self.backing_mem.as_mut_ptr() as usize;
        let mem_end_addr = mem_start_addr + self.backing_mem.len();
        let end_tag_addr = end_tag as usize;

        if end_tag_addr < mem_start_addr + Tags::BeginTag::TAG_SIZE + 1
            || end_tag_addr > mem_end_addr - Tags::EndTag::TAG_SIZE
        {
            return Err(());
        }

        let mem_len = self.backing_mem.len();
        let chunk = &mut self.backing_mem[..mem_len - (mem_end_addr - end_tag_addr)];
        let chunk_len = chunk.len();
        let end_tag = Tags::EndTag::read_from_chunk(chunk);
        let chunk = &mut chunk[chunk_len - end_tag.content_size() - Tags::BeginTag::TAG_SIZE..];
        let begin_tag = Tags::BeginTag::read_from_chunk(chunk);
        let content = unsafe { chunk.as_mut_ptr().add(Tags::BeginTag::TAG_SIZE) };

        assert_eq!(begin_tag.content_size(), end_tag.content_size().into());

        Ok((begin_tag, content, end_tag))
    }

    /// Get the next chunk that immediately follows the given chunk
    pub(crate) fn get_next_chunk(&mut self, chunk: Chunk<Tags>) -> Option<Chunk<Tags>> {
        let (begin_tag, content_ptr, _) = chunk;
        let next_chunk_ptr = unsafe {
            content_ptr
                .add(begin_tag.content_size().into())
                .add(Tags::EndTag::TAG_SIZE)
                .cast::<Tags::BeginTag>()
        };
        self.get_chunk_from_begin(next_chunk_ptr).ok()
    }

    /// Get the chunk that immediately precedes the given chunk
    pub(crate) fn get_prev_chunk(&mut self, chunk: Chunk<Tags>) -> Option<Chunk<Tags>> {
        let prev_end_ptr = unsafe {
            chunk
                .1
                .sub(Tags::BeginTag::TAG_SIZE)
                .sub(Tags::EndTag::TAG_SIZE)
                .cast::<Tags::EndTag>()
        };
        self.get_chunk_from_end(prev_end_ptr).ok()
    }

    /// Get the slice of the backing memory that holds the given chunk
    #[inline]
    fn get_chunk_slice(&mut self, chunk: Chunk<Tags>) -> &mut [u8] {
        let mem_start_addr = self.backing_mem.as_mut_ptr() as usize;

        &mut self.backing_mem[chunk.1 as usize - mem_start_addr - Tags::BeginTag::TAG_SIZE
            ..chunk.1 as usize - mem_start_addr + chunk.0.content_size() + Tags::EndTag::TAG_SIZE]
    }

    /// Get the slice fo the backing memory that holds the given chunks content
    #[inline]
    fn get_content_slice(&mut self, chunk: Chunk<Tags>) -> &mut [u8] {
        let mem_start_addr = self.backing_mem.as_mut_ptr() as usize;
        &mut self.backing_mem[chunk.1 as usize - mem_start_addr
            ..chunk.1 as usize - mem_start_addr + chunk.0.content_size()]
    }

    /// Shift (and shrink) the given chunk by `n` bytes and return a new handle to the shifted chunk.
    ///
    /// This is implemented by adding padding `n` padding bytes to the end of the preceding chunks content and then
    /// moving & updating all tags accordingly.
    ///
    /// If the current chunk cannot be shifted because it is the first chunk of the backing memory, an `Err(())` is
    /// returned.
    ///
    /// # Safety
    /// If the function succeeds, all existing handles to `chunk` as well as potential handles to chunks located
    /// immediately before or after it must not be used after calling this function because the underlying memory
    /// layout changed and those handles are now invalid.
    unsafe fn shift_chunk(&mut self, chunk: Chunk<Tags>, n: usize) -> Result<Chunk<Tags>, ()> {
        assert!(chunk.0.content_size() > n);
        assert_eq!(chunk.0.state(), AllocationMarker::Free);

        let mem_start_addr = self.backing_mem.as_mut_ptr() as usize;

        // if the requested shift is very large, split the free chunk
        if n > Tags::TAGS_SIZE {
            let (_head, tail) = self.split_chunk(chunk, n - Tags::TAGS_SIZE);
            Ok(tail)
        }
        // otherwise, add padding to the previous chunk
        else {
            // update the tags of the previous chunk for its new size
            let prev_chunk = self.get_prev_chunk(chunk).ok_or(())?;
            let new_prev_content_size = prev_chunk.0.content_size() + n;
            Tags::BeginTag::new(new_prev_content_size, prev_chunk.0.state()).write_to_chunk(
                &mut self.backing_mem
                    [(prev_chunk.1 as usize - Tags::BeginTag::TAG_SIZE) - mem_start_addr..],
            );
            Tags::EndTag::new(new_prev_content_size).write_to_chunk(
                &mut self.backing_mem[..(prev_chunk.1 as usize
                    + new_prev_content_size
                    + Tags::EndTag::TAG_SIZE)
                    - mem_start_addr],
            );

            // write new tags for the shifted chunk
            let new_content_size = chunk.0.content_size() - n;
            let new_chunk_addr = chunk.1 as usize - Tags::BeginTag::TAG_SIZE + n;
            Tags::BeginTag::new(new_content_size, AllocationMarker::Free)
                .write_to_chunk(&mut self.backing_mem[new_chunk_addr - mem_start_addr..]);
            Tags::EndTag::new(new_content_size).write_to_chunk(
                &mut self.backing_mem
                    [..new_chunk_addr - mem_start_addr + new_content_size + Tags::TAGS_SIZE],
            );

            // return a new handle to the now shifted chunk
            Ok(self
                .get_chunk_from_begin(new_chunk_addr as *mut Tags::BeginTag)
                .unwrap())
        }
    }

    /// Split the given chunk into `(head, tail)` with `head` having `head_content_size` available content bytes and
    /// `tail` having the remaining bytes of the original chunk.
    ///
    /// Returns the `(head, tail)` tuple of chunks.
    ///
    /// # Safety
    /// All existing handles to `chunk` must not be used after calling this function because the memory layout has
    /// changed which invalidates the handles.
    unsafe fn split_chunk<'a>(
        &mut self,
        chunk: Chunk<Tags>,
        head_content_size: usize,
    ) -> (Chunk<Tags>, Chunk<Tags>) {
        assert!(head_content_size < chunk.0.content_size() - Tags::TAGS_SIZE);
        assert!(head_content_size >= 1);
        assert_eq!(chunk.0.state(), AllocationMarker::Free);

        let chunk_slice = self.get_chunk_slice(chunk);

        // write new tags for the head portion
        Tags::BeginTag::new(head_content_size, AllocationMarker::Free).write_to_chunk(chunk_slice);
        Tags::EndTag::new(head_content_size)
            .write_to_chunk(&mut chunk_slice[..head_content_size + Tags::TAGS_SIZE]);

        // write new tags for the tail portion
        let tail_content_size = chunk.0.content_size() - head_content_size - Tags::TAGS_SIZE;
        Tags::BeginTag::new(tail_content_size, AllocationMarker::Free)
            .write_to_chunk(&mut chunk_slice[head_content_size + Tags::TAGS_SIZE..]);
        Tags::EndTag::new(tail_content_size).write_to_chunk(chunk_slice);

        // re-read chunks from updated backing memory
        let head = self
            .get_chunk_from_begin(unsafe { chunk.1.sub(Tags::BeginTag::TAG_SIZE).cast() })
            .unwrap();
        let tail = self.get_next_chunk(head).unwrap();
        (head, tail)
    }

    /// Mark the given chunk as claimed and return a new updated handle to it
    ///
    /// # Safety
    /// All existing handles to `chunk` must not be used after calling this function because the memory layout has
    /// changed which invalidates the handles.
    unsafe fn mark_chunk_claimed(&mut self, chunk: Chunk<Tags>) -> Chunk<Tags> {
        assert_eq!(chunk.0.state(), AllocationMarker::Free);

        let chunk_slice = self.get_chunk_slice(chunk);

        let begin_tag =
            Tags::BeginTag::new(chunk.0.content_size().into(), AllocationMarker::Allocated);
        begin_tag.write_to_chunk(chunk_slice);
        (begin_tag, chunk.1, chunk.2)
    }

    /// Mark the given chunk as free and return a new updated handle to it
    ///
    /// # Safety
    /// All existing handles to `chunk` must not be used after calling this function because the underlying memory
    /// has changed which invalidates the handles.
    unsafe fn mark_chunk_free(&mut self, chunk: Chunk<Tags>) -> Chunk<Tags> {
        assert_eq!(chunk.0.state(), AllocationMarker::Allocated);

        let chunk_slice = self.get_chunk_slice(chunk);

        let begin_tag = Tags::BeginTag::new(chunk.0.content_size().into(), AllocationMarker::Free);
        begin_tag.write_to_chunk(chunk_slice);
        (begin_tag, chunk.1, chunk.2)
    }

    /// Allocate a chunk from the underlying memory.
    ///
    /// This finds a chunk that is able to hold the requested layout, marks it as used and returns the chunks content.
    fn allocate_chunk(&mut self, layout: Layout) -> Result<&'mem mut [u8], AllocError> {
        let mut chunk = self.get_first_chunk();

        loop {
            // skip chunks that are tagged as Allocated
            if chunk.0.state() == AllocationMarker::Allocated {
                chunk = self
                    .get_next_chunk(chunk)
                    .ok_or(AllocError::InsufficientMemory)?;
                continue;
            }

            // calculate padding that would be required to use this chunk
            let unaligned_addr = chunk.1 as usize;
            let aligned_addr = (unaligned_addr + layout.align() - 1) & !(layout.align() - 1);
            let padding = aligned_addr - unaligned_addr;

            // skip chunks that cannot be used because they are not large enough
            if (chunk.0.content_size()) < layout.size() + padding {
                chunk = self
                    .get_next_chunk(chunk)
                    .ok_or(AllocError::InsufficientMemory)?;
                continue;
            }

            // if we have not skipped any chunks, the current chunk is free and large enough

            // meet the layouts alignment requirement by shifting the chunk if required
            if padding > 0 {
                match unsafe { self.shift_chunk(chunk, padding) } {
                    Ok(new_chunk) => chunk = new_chunk,
                    Err(_) => {
                        // cannot shift chunk because it is the first

                        // if the required padding is large enough so that it can maintain a chunk of its own,
                        // split the current chunk in two so that the tail part is padded correctly.
                        if padding > Tags::TAGS_SIZE {
                            let (_head, tail) =
                                unsafe { self.split_chunk(chunk, padding - Tags::TAGS_SIZE) };
                            chunk = tail;
                        }
                        // if the padding is not large enough to hold a chunk on its own but the current chunk is so
                        // large that it can hold enough padding to reach the next aligned address, split it in two
                        // so that the tail part is padded correctly to that next aligned address
                        else if chunk.0.content_size() > padding + layout.align() {
                            let (_head, tail) = unsafe {
                                self.split_chunk(chunk, padding + layout.align() - Tags::TAGS_SIZE)
                            };
                            chunk = tail
                        }
                        // the chunk is not able to hold the layout so we try the next one
                        else {
                            chunk = self
                                .get_next_chunk(chunk)
                                .ok_or(AllocError::InsufficientMemory)?;
                            continue;
                        }
                    }
                }
                assert_eq!(chunk.1 as usize % layout.align(), 0)
            }

            // meet the layouts size requirement by slicing of a part of the chunk to use
            // (only if the chunk is large enough to hold another allocation later. otherwise just use it directly)
            if chunk.0.content_size() > layout.size() + Tags::TAGS_SIZE {
                let (head, _tail) = unsafe { self.split_chunk(chunk, layout.size()) };
                chunk = head;
            }

            // mark the chunk as claimed
            chunk = unsafe { self.mark_chunk_claimed(chunk) };

            // get the content slice while lifting its lifetime
            // Safety: Lifting the lifetimes is okay because we have ensured that no aliasing can occur by properly
            // tagging the chunk
            let allocation = &mut self.get_content_slice(chunk)[..layout.size()];
            let allocation: &'mem mut [u8] = unsafe { &mut *(allocation as *mut [u8]) };
            return Ok(allocation);
        }
    }

    /// Coalesce two free chunks into one and return a new handle to the new joined chunk.
    ///
    /// **Note:** `chunk1` must immediately precede `chunk2` and both must be free.
    ///
    /// # Safety
    /// All existing handles to `chunk` must not be used after calling this function because the underlying memory
    /// has changed which invalidates the handles.
    unsafe fn coalesce_chunks(&mut self, chunk1: Chunk<Tags>, chunk2: Chunk<Tags>) -> Chunk<Tags> {
        assert_eq!(chunk1.0.state(), AllocationMarker::Free);
        assert_eq!(chunk2.0.state(), AllocationMarker::Free);
        assert_eq!(self.get_next_chunk(chunk1), Some(chunk2));

        // write new tags at begin and end of the merged chunk
        let new_size = chunk1.0.content_size() + chunk2.0.content_size() + Tags::TAGS_SIZE;
        Tags::BeginTag::new(new_size, AllocationMarker::Free)
            .write_to_chunk(self.get_chunk_slice(chunk1));
        Tags::EndTag::new(new_size).write_to_chunk(self.get_chunk_slice(chunk2));

        self.get_chunk_from_content(chunk1.1).unwrap()
    }

    /// Deallocate a chunk that is currently allocated from the backing memory.
    ///
    /// # Safety
    /// The given chunk must be *currently allocated* from this allocator.
    ///
    /// This means that:
    /// - it was previously returned by [`allocate_chunk`](AllocatorState::allocate_chunk)
    /// - it has not yet been deallocated
    unsafe fn deallocate_chunk(&mut self, mut chunk: Chunk<Tags>) {
        chunk = self.mark_chunk_free(chunk);

        // try to coalesce with the next chunk
        if let Some(next_chunk) = self.get_next_chunk(chunk) {
            if next_chunk.0.state() == AllocationMarker::Free {
                chunk = self.coalesce_chunks(chunk, next_chunk);
            }
        }

        // try to coalesce with the previous chunk
        if let Some(prev_chunk) = self.get_prev_chunk(chunk) {
            if prev_chunk.0.state() == AllocationMarker::Free {
                #[allow(unused_assignments)]
                {
                    chunk = self.coalesce_chunks(prev_chunk, chunk);
                }
            }
        }
    }
}

/// A general purpose allocator that attaches boundary tags to handed out memory for bookkeeping.
pub struct BoundaryTagAllocator<'mem, Tags: TagsBinding> {
    pub(super) state: SpinLock<AllocatorState<'mem, Tags>>,
    _tags: PhantomData<Tags>,
}

impl<'mem, Tags: TagsBinding> BoundaryTagAllocator<'mem, Tags> {
    /// Create a new allocator that allocates from the given backing memory.
    pub fn new(backing_mem: &'mem mut [u8]) -> Self {
        assert!(
            backing_mem.len() <= Tags::MAX_CONTENT_SIZE,
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
            state: SpinLock::new(AllocatorState::new(backing_mem)),
            _tags: PhantomData::default(),
        }
    }
}

impl<'mem, Tags: TagsBinding> Allocator<'mem> for BoundaryTagAllocator<'mem, Tags> {
    fn allocate(&self, layout: Layout, init: AllocInit) -> Result<&'mem mut [u8], AllocError> {
        assert!(layout.size() > 0, "must allocate at least 1 byte");
        let allocation = {
            let mut state = self.state.spin_lock();
            state.allocate_chunk(layout)?
        };

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

        Ok(allocation)
    }

    unsafe fn deallocate(&self, data_ptr: *mut u8, layout: Layout) {
        let mut state = self.state.spin_lock();
        let chunk = state
            .get_chunk_from_content(data_ptr)
            .expect("Given data_ptr does not point inside the allocators backing memory");
        assert!(chunk.0.content_size() >= layout.size());
        state.deallocate_chunk(chunk)
    }
}

unsafe impl<'mem, T: TagsBinding> core::alloc::GlobalAlloc for BoundaryTagAllocator<'mem, T> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        match self.allocate(layout, AllocInit::Uninitialized) {
            Ok(slice) => slice.as_mut_ptr(),
            Err(_) => core::ptr::null_mut(),
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.deallocate(ptr, layout)
    }
}
