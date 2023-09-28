extern crate alloc;
extern crate std;

use super::*;
use crate::boundary_tag_alloc::allocator::AllocatorState;
use crate::boundary_tag_alloc::tags::{AllocationState, BeginTag, EndTag};
use crate::{AllocInit, Allocator};
use static_assertions::assert_eq_size;
use std::alloc::Layout;
use std::fmt::{Debug, Formatter};
use std::vec::Vec;
use std::{format, mem, println};

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

#[test]
fn test_padding_area_of_very_large_padding_is_reused() {
    let mut mem = [0u8; 10];
    let alloc = BoundaryTagAllocator::new(&mut mem);

    println!("Before Allocation: {:#?}", alloc.state.borrow());
    let block = alloc
        .allocate(
            Layout::from_size_align(1, 8).unwrap(),
            AllocInit::Data(0x11),
        )
        .unwrap();
    println!("After Allocation:  {:#?}", alloc.state.borrow());

    assert_eq!(block.len(), 1);
    assert_eq!((block.as_ptr() as usize) % 8, 0);
    let alloc_state = alloc.state.borrow();
    assert_eq!(
        alloc_state.backing_mem,
        [
            3,
            AllocationState::Free as u8,
            0,
            0,
            0,
            3,
            1,
            AllocationState::Allocated as u8,
            0x11,
            1
        ]
    );
}
