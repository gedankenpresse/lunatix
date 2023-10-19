extern crate alloc;
extern crate std;

use crate::boundary_tag_alloc::allocator::{AllocatorState, BoundaryTagAllocator};
use crate::boundary_tag_alloc::tags::{
    AllocationMarker, BeginTag, BeginTagU16, EndTag, EndTagU16, TagsBinding, TagsU16, TagsU8,
    TagsUsize,
};
use crate::boundary_tag_alloc::{TagsU32, TagsU64};
use crate::{stack_alloc, AllocInit, Allocator, Box};
use std::alloc::Layout;
use std::fmt::{Debug, Formatter};
use std::vec::Vec;
use std::{format, mem, println};

impl<Tags: TagsBinding> Debug for AllocatorState<'_, Tags> {
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
                        let begin_tag = Tags::BeginTag::read_from_chunk(block);
                        let end_tag = Tags::EndTag::read_from_chunk(block);
                        format!(
                            "[<{} {}> ... <{}>]",
                            begin_tag.content_size(),
                            match begin_tag.state() {
                                AllocationMarker::Free => "Free",
                                AllocationMarker::Allocated => "Used",
                            },
                            end_tag.content_size()
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

#[test]
fn test_initial_tags() {
    let mut mem = [0u8; 8];
    let alloc: BoundaryTagAllocator<TagsU8> = BoundaryTagAllocator::new(&mut mem);
    let alloc_state = alloc.state.borrow();
    println!("{:#?}", alloc_state);
    assert_eq!(
        alloc_state.backing_mem,
        [5, AllocationMarker::Free as u8, 0, 0, 0, 0, 0, 5]
    );
}

#[test]
fn test_alloc_one_u8() {
    let mut mem = [0u8; 8];
    let alloc: BoundaryTagAllocator<TagsU8> = BoundaryTagAllocator::new(&mut mem);

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
            AllocationMarker::Allocated as u8,
            0x55,
            1,
            1,
            AllocationMarker::Free as u8,
            0,
            1
        ]
    );
}

#[test]
fn test_alloc_multiple_u8() {
    let mut mem = [0u8; 16];
    let alloc: BoundaryTagAllocator<TagsU8> = BoundaryTagAllocator::new(&mut mem);

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
            AllocationMarker::Allocated as u8,
            0x11,
            1,
            1,
            AllocationMarker::Allocated as u8,
            0x22,
            1,
            1,
            AllocationMarker::Allocated as u8,
            0x33,
            1,
            1,
            AllocationMarker::Free as u8,
            0,
            1
        ]
    );
}

#[test]
fn test_alloc_one_u32() {
    let mut mem = [0u8; 13];
    let alloc: BoundaryTagAllocator<TagsU8> = BoundaryTagAllocator::new(&mut mem);

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
            3,
            AllocationMarker::Free as u8,
            0,
            0,
            0,
            3,
            mem::size_of::<u32>() as u8,
            AllocationMarker::Allocated as u8,
            0x11,
            0x11,
            0x11,
            0x11,
            mem::size_of::<u32>() as u8,
        ]
    );
}

#[test]
fn test_alloc_last_block() {
    let mut mem = [0u8; 4];
    let alloc: BoundaryTagAllocator<TagsU8> = BoundaryTagAllocator::new(&mut mem);
    println!("Before Allocation: {:#?}", alloc.state.borrow());
    let _block = alloc
        .allocate(Layout::new::<u8>(), AllocInit::Data(0x11))
        .unwrap();
    println!("After Allocation:  {:#?}", alloc.state.borrow());

    let alloc_state = alloc.state.borrow();
    assert_eq!(
        alloc_state.backing_mem,
        [1, AllocationMarker::Allocated as u8, 0x11, 1]
    );
}

#[test]
fn test_alloc_one_with_alignment() {
    let mut mem = [0u8; 16];
    let alloc: BoundaryTagAllocator<TagsU8> = BoundaryTagAllocator::new(&mut mem);

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
    let alloc: BoundaryTagAllocator<TagsU8> = BoundaryTagAllocator::new(&mut mem);

    println!("Before Allocation: {:#?}", alloc.state.borrow());
    let block = alloc
        .allocate(Layout::new::<u8>(), AllocInit::Data(0x11))
        .unwrap();
    println!("After Allocation:  {:#?}", alloc.state.borrow());

    assert_eq!(block.len(), 1);
    let alloc_state = alloc.state.borrow();
    assert_eq!(
        alloc_state.backing_mem,
        [2, AllocationMarker::Allocated as u8, 0x11, 0, 2]
    );
}

#[test]
fn test_u16_tag() {
    let mut mem = [0u8; 32];
    let alloc: BoundaryTagAllocator<TagsU16> = BoundaryTagAllocator::new(&mut mem);

    println!("Initial: {:#?}", alloc.state.borrow());
    let block1 = alloc
        .allocate(Layout::new::<u8>(), AllocInit::Data(0x11))
        .unwrap();
    println!("After allocation 1: {:#?}", alloc.state.borrow());
    let block2 = alloc
        .allocate(Layout::new::<u8>(), AllocInit::Data(0x22))
        .unwrap();
    println!("After allocation 2: {:#?}", alloc.state.borrow());

    assert_eq!(block1.len(), mem::size_of::<u8>());
    assert_eq!(block2.len(), mem::size_of::<u8>());
}

#[test]
fn test_u32_tag() {
    let mut mem = [0u8; 32];
    let alloc: BoundaryTagAllocator<TagsU32> = BoundaryTagAllocator::new(&mut mem);

    println!("Initial: {:#?}", alloc.state.borrow());
    let block1 = alloc
        .allocate(Layout::new::<u8>(), AllocInit::Data(0x11))
        .unwrap();
    println!("After allocation 1: {:#?}", alloc.state.borrow());
    let block2 = alloc
        .allocate(Layout::new::<u8>(), AllocInit::Data(0x22))
        .unwrap();
    println!("After allocation 2: {:#?}", alloc.state.borrow());

    assert_eq!(block1.len(), mem::size_of::<u8>());
    assert_eq!(block2.len(), mem::size_of::<u8>());
}

#[test]
fn test_u64_tag() {
    let mut mem = [0u8; 64];
    let alloc: BoundaryTagAllocator<TagsU64> = BoundaryTagAllocator::new(&mut mem);

    println!("Initial: {:#?}", alloc.state.borrow());
    let block1 = alloc
        .allocate(Layout::new::<u8>(), AllocInit::Data(0x11))
        .unwrap();
    println!("After allocation 1: {:#?}", alloc.state.borrow());
    let block2 = alloc
        .allocate(Layout::new::<u8>(), AllocInit::Data(0x22))
        .unwrap();
    println!("After allocation 2: {:#?}", alloc.state.borrow());

    assert_eq!(block1.len(), mem::size_of::<u8>());
    assert_eq!(block2.len(), mem::size_of::<u8>());
}

#[test]
fn test_usize_tags() {
    let mut mem = [0u8; 64];
    let alloc: BoundaryTagAllocator<TagsUsize> = BoundaryTagAllocator::new(&mut mem);

    println!("Initial: {:#?}", alloc.state.borrow());
    let block1 = alloc
        .allocate(Layout::new::<u8>(), AllocInit::Data(0x11))
        .unwrap();
    println!("After allocation 1: {:#?}", alloc.state.borrow());
    let block2 = alloc
        .allocate(Layout::new::<u8>(), AllocInit::Data(0x22))
        .unwrap();
    println!("After allocation 2: {:#?}", alloc.state.borrow());

    assert_eq!(block1.len(), mem::size_of::<u8>());
    assert_eq!(block2.len(), mem::size_of::<u8>());
}

#[test]
fn test_second_chunk_needing_padding() {
    let mut mem = [0u8; 16];
    let alloc: BoundaryTagAllocator<TagsU8> = BoundaryTagAllocator::new(&mut mem);

    println!("Initial State: {:#?}", alloc.state.borrow());
    let _block1 = alloc
        .allocate(
            Layout::from_size_align(1, 1).unwrap(),
            AllocInit::Data(0x55),
        )
        .unwrap();
    println!("After Allocation 1:  {:#?}", alloc.state.borrow());
    let _block2 = alloc
        .allocate(
            Layout::from_size_align(1, 4).unwrap(),
            AllocInit::Data(0x55),
        )
        .unwrap();
    println!("After Allocation 2: {:#?}", alloc.state.borrow());
}

#[test]
fn test_dealloc_one() {
    let mut mem = [0u8; 8];
    let alloc: BoundaryTagAllocator<TagsU8> = BoundaryTagAllocator::new(&mut mem);

    println!("Before Allocation: {:#?}", alloc.state.borrow());
    let layout = Layout::new::<u8>();
    let block = alloc.allocate(layout, AllocInit::Data(0x55)).unwrap();
    println!("After Allocation:  {:#?}", alloc.state.borrow());
    unsafe { alloc.deallocate(block.as_mut_ptr(), layout) };
    println!("After Deallocation: {:#?}", alloc.state.borrow());

    assert_eq!(block.len(), mem::size_of::<u8>());
    let alloc_state = alloc.state.borrow();
    assert_eq!(
        alloc_state.backing_mem,
        [5, AllocationMarker::Free as u8, 0x55, 1, 1, 1, 0, 5]
    );
}

#[test]
fn test_get_first_chunk() {
    // arrange
    let mut mem = [0u8; 8];
    let begin_tag = BeginTagU16::new(3, AllocationMarker::Free);
    let end_tag = EndTagU16::new(3);
    begin_tag.write_to_chunk(&mut mem);
    end_tag.write_to_chunk(&mut mem);
    let mut state = AllocatorState::<TagsU16>::new(&mut mem);

    // act
    let chunk = state.get_first_chunk();

    // assert
    assert_eq!(chunk.0, begin_tag);
    assert_eq!(chunk.1, unsafe {
        mem.as_mut_ptr().add(BeginTagU16::TAG_SIZE)
    });
    assert_eq!(chunk.2, end_tag);
}

#[test]
fn test_get_chunk_from_begin_tag() {
    // arrange
    let mut mem = [0u8; 8];
    let begin_ptr = mem.as_mut_ptr().cast();
    let begin_tag = BeginTagU16::new(3, AllocationMarker::Free);
    let end_tag = EndTagU16::new(3);
    begin_tag.write_to_chunk(&mut mem);
    end_tag.write_to_chunk(&mut mem);
    let mut state = AllocatorState::<TagsU16>::new(&mut mem);

    // act
    let chunk = state.get_chunk_from_begin(begin_ptr).unwrap();

    // assert
    assert_eq!(chunk.0, begin_tag);
    assert_eq!(chunk.1, unsafe {
        mem.as_mut_ptr().add(BeginTagU16::TAG_SIZE)
    });
    assert_eq!(chunk.2, end_tag);
}

#[test]
fn test_get_chunk_from_content_ptr() {
    // arrange
    let mut mem = [0u8; 8];
    let content_ptr = unsafe { mem.as_mut_ptr().add(BeginTagU16::TAG_SIZE) };
    let begin_tag = BeginTagU16::new(3, AllocationMarker::Free);
    let end_tag = EndTagU16::new(3);
    begin_tag.write_to_chunk(&mut mem);
    end_tag.write_to_chunk(&mut mem);
    let mut state = AllocatorState::<TagsU16>::new(&mut mem);

    // act
    let chunk = state.get_chunk_from_content(content_ptr).unwrap();

    // assert
    assert_eq!(chunk.0, begin_tag);
    assert_eq!(chunk.1, content_ptr);
    assert_eq!(chunk.2, end_tag);
}

#[test]
fn test_get_chunk_from_end_tag() {
    // arrange
    let mut mem = [0u8; 8];
    let end_ptr = unsafe { mem.as_mut_ptr().add(BeginTagU16::TAG_SIZE).add(3).cast() };
    let begin_tag = BeginTagU16::new(3, AllocationMarker::Free);
    let end_tag = EndTagU16::new(3);
    begin_tag.write_to_chunk(&mut mem);
    end_tag.write_to_chunk(&mut mem);
    let mut state = AllocatorState::<TagsU16>::new(&mut mem);

    // act
    let chunk = state.get_chunk_from_end(end_ptr).unwrap();

    // assert
    assert_eq!(chunk.0, begin_tag);
    assert_eq!(chunk.1, unsafe {
        mem.as_mut_ptr().add(BeginTagU16::TAG_SIZE)
    });
    assert_eq!(chunk.2, end_tag);
}

#[test]
fn test_with_box() {
    stack_alloc!(allocator, 16, BoundaryTagAllocator<TagsU16>);
    let b = Box::new(0x55u8, &allocator).unwrap();
    assert_eq!(*b, 0x55u8);
    drop(b);
}

#[test]
fn test_large_padding_is_reused() {
    // arrange
    let mut mem = [0u8; TagsU8::MAX_CONTENT_SIZE];
    let alloc = BoundaryTagAllocator::<TagsU8>::new(&mut mem);
    let _block1 = alloc
        .allocate(Layout::new::<u8>(), AllocInit::Data(0x11))
        .unwrap();
    println!("After Setup: {:#?}", alloc.state.borrow());

    // act
    let block2 = alloc
        .allocate(
            Layout::from_size_align(1, 128).unwrap(),
            AllocInit::Data(0x55),
        )
        .unwrap();
    println!("After Allocation: {:#?}", alloc.state.borrow());

    // assert
    assert_eq!(block2.as_mut_ptr() as usize % 128, 0);
    assert_eq!(block2[0], 0x55);
    assert_eq!(&mem[0..4], [1, AllocationMarker::Allocated as u8, 0x11, 1,]);
    assert_eq!(&mem[4..6], [7, AllocationMarker::Free as u8]);
    assert_eq!(
        &mem[14..18],
        [1, AllocationMarker::Allocated as u8, 0x55, 1,]
    )
}
