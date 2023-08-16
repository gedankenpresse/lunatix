//! Abstraction library for working with capability derivations

#![no_std]

extern crate alloc;

mod cap_counted;
pub mod caps;
mod correspondence;
mod cursors;
mod node;
mod tree;

pub use correspondence::Correspondence;
pub use cursors::{AliasingError, CursorHandle, CursorSet, OutOfCursorsError};
pub use node::{TreeNodeData, TreeNodeOps};
pub use tree::DerivationTree;

#[cfg(test)]
pub(crate) use test::assume_init_box;

#[cfg(test)]
mod test {
    extern crate std;

    use alloc::boxed::Box as StdBox;
    use core::mem::MaybeUninit;

    pub unsafe fn assume_init_box<T>(value: StdBox<MaybeUninit<T>>) -> StdBox<T> {
        let raw = StdBox::into_raw(value);
        StdBox::from_raw(raw as *mut T)
    }

    pub mod node_tests {
        use crate::{Correspondence, TreeNodeData, TreeNodeOps};

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
    }

    pub mod full_capability_tests {
        use crate::caps::test_union::{CSpaceIface, TestCapTag, TestCapUnion, ValueCapIface};
        use crate::caps::CapabilityIface;
        use crate::test::assume_init_box;
        use crate::{DerivationTree, TreeNodeOps};
        use alloc::boxed::Box as StdBox;
        use alloc::vec;
        use alloc::vec::Vec;
        use allocators::bump_allocator::{BumpAllocator, ForwardBumpingAllocator};
        use core::mem::MaybeUninit;

        #[test]
        fn full_tree_with_cspaces() {
            // arrange
            let mem = Vec::leak::<'static>(vec![0; 4096]);
            let allocator = StdBox::leak::<'static>(StdBox::new(ForwardBumpingAllocator::new(mem)));
            let mut loc = StdBox::new(MaybeUninit::uninit());

            // act
            // initialize a tree with a CSpace node as root
            let tree = unsafe {
                DerivationTree::init_with_root_value(&mut loc, TestCapUnion::default());
                assume_init_box(loc)
            };
            let mut cursor = tree.get_root_cursor().unwrap();
            let mut cspace_cap = cursor.get_exclusive().unwrap();
            CSpaceIface.init(&mut cspace_cap, (allocator, 4));

            if cspace_cap.tag == TestCapTag::CSpace {
                unsafe {
                    // create a new UsizeCap and store it as a derivation of the CSpace (this semantically does not make sense but we want to test)
                    let mut usize_cap = &mut *cspace_cap.payload.cspace.lookup_raw(0).unwrap();
                    ValueCapIface.init(&mut usize_cap, 42);
                    cspace_cap.insert_derivation(usize_cap);
                    assert!(!usize_cap.get_tree_data().is_unlinked());
                    let mut usize_cursor = tree.get_node(usize_cap as *mut _).unwrap();
                    let usize_cap = usize_cursor.get_shared().unwrap();

                    // copy the UsizeCap
                    let usize_cap2 = &mut *cspace_cap.payload.cspace.lookup_raw(1).unwrap();
                    ValueCapIface.copy(&usize_cap, usize_cap2);
                    assert!(!usize_cap2.get_tree_data().is_unlinked());
                }
            }

            // assert
            assert_eq!(cspace_cap.tag, TestCapTag::CSpace);
        }
    }
}
