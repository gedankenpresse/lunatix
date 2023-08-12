//! Abstraction library for working with capability derivations

#![no_std]

extern crate alloc;

mod correspondence;
mod cspace;
mod cursors;
mod node;
mod tree;

pub use correspondence::Correspondence;
pub use cursors::{AliasingError, CursorHandle, CursorSet, OutOfCursorsError};
pub use node::{TreeNodeData, TreeNodeOps};
pub use tree::DerivationTree;

#[cfg(test)]
pub(crate) use test::{assume_init_box, TestNode};

#[cfg(test)]
mod test {
    extern crate std;

    use crate::{Correspondence, TreeNodeData, TreeNodeOps};
    use alloc::boxed::Box;
    use core::mem::MaybeUninit;

    pub struct TestNode {
        pub tree_data: TreeNodeData<Self>,
        pub value: usize,
    }

    impl TestNode {
        pub fn new(value: usize) -> Self {
            Self {
                tree_data: unsafe { TreeNodeData::new() },
                value,
            }
        }
    }

    impl TreeNodeOps for TestNode {
        fn get_tree_data(&self) -> &TreeNodeData<Self> {
            &self.tree_data
        }
    }

    impl Correspondence for TestNode {
        fn corresponds_to(&self, other: &Self) -> bool {
            false
        }
    }

    pub unsafe fn assume_init_box<T>(value: Box<MaybeUninit<T>>) -> Box<T> {
        let raw = Box::into_raw(value);
        Box::from_raw(raw as *mut T)
    }

    #[test]
    fn full_tree_with_cspaces() {
        // arrange
    }
}
