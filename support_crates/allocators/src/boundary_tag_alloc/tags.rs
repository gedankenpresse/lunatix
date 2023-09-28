use core::fmt::Debug;
use core::mem;

// TODO Implement multiple tag types with different sized size fields (u16, u32, u64, usize)

/// Whether a chunk is allocated or free
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
#[repr(u8)]
pub(super) enum AllocationMarker {
    Free = 1,
    Allocated = 2,
}

/// A trait binding BeginTag and EndTag implementations together
pub(super) trait TagsBinding {
    /// How much memory the allocator must reserve to store both tags.
    const TAGS_SIZE: usize;

    type BeginTag: BeginTag;
    type EndTag: EndTag;
}

/// A trait that all possible sizes of begin-tags implement.
pub(super) trait BeginTag {
    /// The type used for storing the size of a chunks content.
    type SizeT: Into<usize> + TryFrom<usize>;

    /// How much memory the allocator must reserve at the start of a chunk for storing this tag type.
    const TAG_SIZE: usize;

    /// The maximum amount of bytes for which bookkeeping information can be stored using this tag type.
    const MAX_CONTENT_SIZE: usize;

    fn new(content_size: usize, state: AllocationMarker) -> Self;

    /// The size of the content that is stored in the chunk which is governed by this tag.
    fn content_size(&self) -> Self::SizeT;

    /// Whether the chunk governed by this tag is currently allocated or free.
    fn state(&self) -> AllocationMarker;

    /// Read the tag from a chunk.
    /// The tag is expected to be located at the first few bytes of the chunk.
    fn read_from_chunk(chunk: &[u8]) -> Self;

    /// Write the tag data into the first few bytes of the chunk.
    fn write_to_chunk(&self, chunk: &mut [u8]);
}

pub(super) trait EndTag {
    /// The type used for storing the size of a chunks content.
    type SizeT: Into<usize>;

    /// How much memory the allocator must reserve at the start of a chunk for storing this tag type.
    const TAG_SIZE: usize;

    fn new(content_size: usize) -> Self;

    /// The size of the content that is stored in the chunk which is governed by this tag.
    fn content_size(&self) -> Self::SizeT;

    /// Read the tag from a chunk.
    /// The tag is expected to be located at the last few bytes of the chunk.
    fn read_from_chunk(chunk: &[u8]) -> Self;

    /// Write the tag data into the last few bytes of the chunk.
    fn write_to_chunk(&self, chunk: &mut [u8]);
}

/// Create a begin-tag and end-tag type based on given names and an underlying number type.
macro_rules! make_tag_type {
    ($begin_name:ident, $end_name:ident, $binding_name:ident, $size_t:ty) => {
        #[doc = "A type that instructs the allocator to use `"]
        #[doc = stringify!($size_t)]
        #[doc = "` based tags"]
        pub(super) struct $binding_name;

        impl TagsBinding for $binding_name {
            const TAGS_SIZE: usize = $begin_name::TAG_SIZE + $end_name::TAG_SIZE;

            type BeginTag = $begin_name;
            type EndTag = $end_name;
        }

        #[doc = "A begin-tag that uses a `"]
        #[doc = stringify!($size_t)]
        #[doc = "` for storing the content size of a chunk"]
        #[derive(Debug, Eq, PartialEq)]
        #[repr(C)]
        pub(super) struct $begin_name {
            // TODO Make fields private so that access methods are used
            pub content_size: $size_t,
            pub state: AllocationMarker,
        }

        impl BeginTag for $begin_name {
            type SizeT = $size_t;

            const TAG_SIZE: usize = mem::size_of::<$begin_name>();

            const MAX_CONTENT_SIZE: usize = <$size_t>::MAX as usize;

            fn new(content_size: usize, state: AllocationMarker) -> Self {
                Self {
                    content_size: content_size as $size_t,
                    state,
                }
            }

            fn content_size(&self) -> Self::SizeT {
                self.content_size
            }

            fn state(&self) -> AllocationMarker {
                self.state
            }

            fn read_from_chunk(chunk: &[u8]) -> Self {
                assert!(
                    chunk.len() >= Self::TAG_SIZE,
                    "chunk is not large enough to contain a begin-tag"
                );

                const FREE: u8 = AllocationMarker::Free as u8;
                const ALLOCATED: u8 = AllocationMarker::Allocated as u8;

                // Safety: We have already verified that the chunk is large enough and that the stored tag is valid.
                Self {
                    content_size: <$size_t>::from_ne_bytes(
                        (&chunk[0..Self::TAG_SIZE - 1]).try_into().unwrap(),
                    ),
                    state: match chunk[Self::TAG_SIZE - 1] {
                        FREE => AllocationMarker::Free,
                        ALLOCATED => AllocationMarker::Allocated,
                        _ => panic!("chunk does not contain a valid allocation marker"),
                    },
                }
            }

            fn write_to_chunk(&self, chunk: &mut [u8]) {
                chunk[0..Self::TAG_SIZE - 1].copy_from_slice(&self.content_size.to_ne_bytes());
                chunk[Self::TAG_SIZE - 1] = self.state as u8;
            }
        }

        #[doc = "An end-tag that uses a `"]
        #[doc = stringify!($size_t)]
        #[doc = "` for storing the content size of a chunk"]
        #[derive(Debug, Eq, PartialEq)]
        #[repr(C)]
        pub(super) struct $end_name {
            // TODO Make fields private so that access methods are used
            pub content_size: $size_t,
        }

        impl EndTag for $end_name {
            type SizeT = $size_t;

            const TAG_SIZE: usize = mem::size_of::<$end_name>();

            fn new(content_size: usize) -> Self {
                Self {
                    content_size: content_size as $size_t,
                }
            }

            fn content_size(&self) -> Self::SizeT {
                self.content_size
            }

            fn read_from_chunk(chunk: &[u8]) -> Self {
                assert!(
                    chunk.len() >= Self::TAG_SIZE,
                    "chunk is not large enough to contain a begin-tag"
                );

                // Safety: We have already verified that the chunk is large enough and that the stored tag is valid.
                Self {
                    content_size: <$size_t>::from_ne_bytes(
                        (&chunk[0..Self::TAG_SIZE]).try_into().unwrap(),
                    ),
                }
            }

            fn write_to_chunk(&self, chunk: &mut [u8]) {
                let chunk_len = chunk.len();
                chunk[chunk_len - Self::TAG_SIZE..]
                    .copy_from_slice(&self.content_size.to_ne_bytes());
            }
        }
    };
}

make_tag_type!(BeginTagU8, EndTagU8, TagsU8, u8);
make_tag_type!(BeginTagU16, EndTagU16, TagsU16, u16);
make_tag_type!(BeginTagUsize, EndTagUsize, TagsUsize, usize);
