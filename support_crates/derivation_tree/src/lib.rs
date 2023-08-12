//! Abstraction library for working with capability derivations

#![no_std]

extern crate alloc;

mod correspondence;
mod cspace;
mod cursors;
mod node;
mod tree;
mod uninit;


pub use correspondence::CapabilityOps;
pub use cursors::{AliasingError, CursorHandle, CursorSet, OutOfCursorsError};
pub use node::{TreeNodeData, TreeNodeOps};
pub use tree::DerivationTree;

#[cfg(test)]
pub(crate) use test::{assume_init_box, TestNode};

#[cfg(test)]
mod test {
    extern crate std;

    use crate::correspondence::Correspondence;
    use crate::{CapabilityOps, TreeNodeData, TreeNodeOps};
    use crate::cspace::CSpace;
    use crate::uninit::Uninit;
    use crate::tree::DerivationTree;
    use allocators::Allocator;
    use allocators::bump_allocator::BumpAllocator;
    use alloc::boxed::Box as StdBox;
    use alloc::vec::Vec;
    use alloc::vec;
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

    impl CapabilityOps for TestNode {
        fn cap_copy(&self) {
            todo!()
        }

        fn destroy(&self) {}
    }

    pub enum TestCap<'alloc, 'mem, A: Allocator<'mem>> {
        CSpace(CSpace<'alloc, 'mem, A, TestCap<'alloc, 'mem, A>>),
        Uninit(Uninit<TestCap<'alloc, 'mem, A>>),
    }

    impl<'mem, A: Allocator<'mem>> TreeNodeOps for TestCap<'_, 'mem, A> {
        fn get_tree_data(&self) -> &TreeNodeData<Self> {
            match self {
                TestCap::CSpace(cspace) => &cspace.tree_data,
                TestCap::Uninit(uninit) => &uninit.tree_data,
            }
        }
    }

    impl<'mem, A: Allocator<'mem>> Correspondence for TestCap<'_, 'mem, A> {
        fn corresponds_to(&self, other: &Self) -> bool {
            match (self, other) {
                (TestCap::CSpace(_), TestCap::CSpace(_)) => todo!(),
                (TestCap::Uninit(_), TestCap::Uninit(_)) => todo!(),
                _ => false,
            }
        }
    }

    impl<'mem, A: Allocator<'mem>> CapabilityOps for TestCap<'_, 'mem, A> {
        fn cap_copy(&self) {
            todo!()
        }

        fn destroy(&self) {
            todo!()
        }
    }

    pub unsafe fn assume_init_box<T>(value: StdBox<MaybeUninit<T>>) -> StdBox<T> {
        let raw = StdBox::into_raw(value);
        StdBox::from_raw(raw as *mut T)
    }

    #[test]
    fn full_tree_with_cspaces() {
        // arrange
        use allocators::bump_allocator::ForwardBumpingAllocator;
        type Cap<'alloc, 'mem> = TestCap<'alloc, 'mem, ForwardBumpingAllocator<'mem>>;
        const BYTES: usize = 2048;
        let mut mem: Vec<u8> = vec![0; BYTES];
        let allocator = StdBox::new(ForwardBumpingAllocator::new(&mut mem[..]));
        let mut loc = StdBox::new(MaybeUninit::uninit());

        // act
        let tree = unsafe {
            DerivationTree::init_with_root_value(&mut loc, Cap::Uninit(Uninit::new()));
            assume_init_box(loc)
        };

        let cursor = tree.get_root_cursor().unwrap();
        todo!();
    }
}
