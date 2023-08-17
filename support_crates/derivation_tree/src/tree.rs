use crate::cursors::{CursorHandle, CursorSet, OutOfCursorsError};
use crate::node::TreeNodeOps;
use crate::tree_iter::NextNodeIterator;
use core::mem::MaybeUninit;
use core::ptr::{addr_of, addr_of_mut};

/// A intrinsic collection for tracking nodes that are derived from each other in a tree-like structure.
pub struct DerivationTree<T: TreeNodeOps> {
    root_node: T,
    cursors: CursorSet<T>,
}

impl<T: TreeNodeOps> DerivationTree<T> {
    /// TODO
    ///
    /// # Safety
    /// - This function may only be called at most once because it initializes the provided memory location
    ///   and must be properly dropped too.
    /// - After calling this function, [`assume_init()`](MaybeUninit::assume_init) must be called on `loc`.
    pub unsafe fn init_with_root_value(loc: &mut MaybeUninit<Self>, root_value: T) {
        // create new CursorSet at the correct field location
        CursorSet::init(
            (addr_of_mut!((*loc.as_mut_ptr()).cursors) as *mut MaybeUninit<CursorSet<T>>)
                .as_mut()
                .unwrap(),
        );

        // create a new root node at the correct field location
        root_value.get_tree_data().depth.set(1);
        addr_of_mut!((*loc.as_mut_ptr()).root_node).write(root_value);

        // initialize the root node correctly so that it points to the collections CursorSet
        let DerivationTree { root_node, cursors } = loc.assume_init_mut();
        root_node
            .get_tree_data()
            .assign_cursor_set(cursors as *mut CursorSet<_>);
    }

    /// Try to get a cursor to the root tree node.
    ///
    /// The returned cursor is in [`Inactive`](crate::cursors::CursorData::Inactive) state and must still be locked
    /// to actually use the node.
    pub fn get_root_cursor(&self) -> Result<CursorHandle<T>, OutOfCursorsError> {
        let cursor = self.cursors.get_free_cursor()?;
        cursor.select_node(&self.root_node as *const _ as *mut _);
        Ok(cursor)
    }

    /// Try to get a cursor to the given node.
    ///
    /// This can be used to access a node safely if one already knows about e.g. after inserting it into the tree.
    pub fn get_node(&self, node: *mut T) -> Result<CursorHandle<T>, OutOfCursorsError> {
        let cursor = self.cursors.get_free_cursor()?;
        cursor.select_node(node);
        Ok(cursor)
    }

    /// Get an iterator over all nodes in the tree.
    ///
    /// *Note:* The iterator only hands out raw pointers to the trees nodes and a valid handle must be obtained
    /// by calling [`get_node()`](Self::get_node) to safely use it.
    pub fn iter(&self) -> NextNodeIterator<T> {
        // TODO Fix const2mut cast
        NextNodeIterator::from_starting_node(addr_of!(self.root_node) as *mut _)
    }
}

#[cfg(test)]
mod test {
    extern crate std;

    use super::*;
    use crate::assume_init_box;
    use crate::cursors::CursorRefMut;
    use crate::test::node_tests::TestNode;
    use crate::tree::DerivationTree;
    use alloc::boxed::Box;

    #[test]
    fn test_derivation_tree_creation() {
        // arrange
        let mut loc = Box::new(MaybeUninit::uninit());

        // act
        let tree = unsafe {
            DerivationTree::init_with_root_value(&mut loc, TestNode::new(42));
            assume_init_box(loc)
        };

        // assert
        assert!(
            !tree.root_node.get_tree_data().cursors.get().is_null(),
            "cursors pointer is still uninitialized"
        )
    }

    #[test]
    fn test_get_root_node_cursor() {
        // arrange
        let mut loc = Box::new(MaybeUninit::uninit());
        let tree = unsafe {
            DerivationTree::init_with_root_value(&mut loc, TestNode::new(42));
            assume_init_box(loc)
        };

        // act
        let cursor = tree.get_root_cursor();

        // assert
        assert!(cursor.is_ok())
    }

    #[test]
    fn test_get_overlapping_cursors() {
        // arrange
        let mut loc = Box::new(MaybeUninit::uninit());
        let tree = unsafe {
            DerivationTree::init_with_root_value(&mut loc, TestNode::new(42));
            assume_init_box(loc)
        };

        // act
        let cursor1 = tree.get_root_cursor();
        let cursor2 = tree.get_root_cursor();

        // assert
        assert!(cursor1.is_ok());
        assert!(cursor2.is_ok());
    }

    #[test]
    fn test_get_one_exclusive_ref() {
        // arrange
        let mut loc = Box::new(MaybeUninit::uninit());
        let tree = unsafe {
            DerivationTree::init_with_root_value(&mut loc, TestNode::new(42));
            assume_init_box(loc)
        };

        // act
        let mut cursor = tree.get_root_cursor().unwrap();
        let node = cursor.get_exclusive();

        // assert
        assert!(node.is_ok());
        assert_eq!(node.unwrap().value, 42);
    }

    #[test]
    fn test_get_two_exclusive_refs_fails() {
        // arrange
        let mut loc = Box::new(MaybeUninit::uninit());
        let tree = unsafe {
            DerivationTree::init_with_root_value(&mut loc, TestNode::new(42));
            assume_init_box(loc)
        };

        // act
        let mut cursor1 = tree.get_root_cursor().unwrap();
        let mut cursor2 = tree.get_root_cursor().unwrap();
        let node1 = cursor1.get_exclusive();
        let node2 = cursor2.get_exclusive();

        // assert
        assert!(node1.is_ok());
        assert!(node2.is_err());
    }

    #[test]
    fn test_get_two_shared_refs() {
        // arrange
        let mut loc = Box::new(MaybeUninit::uninit());
        let tree = unsafe {
            DerivationTree::init_with_root_value(&mut loc, TestNode::new(42));
            assume_init_box(loc)
        };

        // act
        let mut cursor1 = tree.get_root_cursor().unwrap();
        let mut cursor2 = tree.get_root_cursor().unwrap();
        let node1 = cursor1.get_shared();
        let node2 = cursor2.get_shared();

        // assert
        assert!(node1.is_ok());
        assert!(node2.is_ok());
        assert_eq!(node1.unwrap().value, 42);
        assert_eq!(node2.unwrap().value, 42);
    }

    #[test]
    fn test_get_shared_then_exclusive_ref_fails() {
        // arrange
        let mut loc = Box::new(MaybeUninit::uninit());
        let tree = unsafe {
            DerivationTree::init_with_root_value(&mut loc, TestNode::new(42));
            assume_init_box(loc)
        };

        // act
        let mut cursor1 = tree.get_root_cursor().unwrap();
        let mut cursor2 = tree.get_root_cursor().unwrap();
        let node1 = cursor1.get_shared();
        let node2 = cursor2.get_exclusive();

        // assert
        assert!(node1.is_ok());
        assert!(node2.is_err());
    }

    #[test]
    fn test_get_exclusive_then_shared_ref_fails() {
        // arrange
        let mut loc = Box::new(MaybeUninit::uninit());
        let tree = unsafe {
            DerivationTree::init_with_root_value(&mut loc, TestNode::new(42));
            assume_init_box(loc)
        };

        // act
        let mut cursor1 = tree.get_root_cursor().unwrap();
        let mut cursor2 = tree.get_root_cursor().unwrap();
        let node1 = cursor1.get_exclusive();
        let node2 = cursor2.get_shared();

        // assert
        assert!(node1.is_ok());
        assert!(node2.is_err());
    }

    #[test]
    fn test_cursor_duplication() {
        // arrange
        let mut loc = Box::new(MaybeUninit::uninit());
        let tree = unsafe {
            DerivationTree::init_with_root_value(&mut loc, TestNode::new(42));
            assume_init_box(loc)
        };
        let mut cursor1 = tree.get_root_cursor().unwrap();
        let node1 = cursor1.get_exclusive().unwrap();

        // act
        let mut cursor2 = CursorRefMut::duplicate(&node1).unwrap();

        // assert
        assert!(cursor2.get_exclusive().is_err()); // this is only an error if the second cursor refers to the same node (which is exclusively locked)
    }

    #[test]
    fn test_insert_1_copy() {
        // arrange
        let mut loc = Box::new(MaybeUninit::uninit());
        let tree = unsafe {
            DerivationTree::init_with_root_value(&mut loc, TestNode::new(42));
            assume_init_box(loc)
        };

        // act
        let mut new_node = TestNode::new(42);
        unsafe {
            tree.root_node.insert_copy(&mut new_node);
        }

        // assert
        assert!(!new_node.tree_data.is_not_in_tree());
        assert_eq!(new_node.tree_data.depth.get(), 1);
        assert!(!new_node.tree_data.prev.get().is_null());
        assert!(new_node.tree_data.next.get().is_null());
        assert!(new_node.is_last_copy());
        assert!(!tree.root_node.has_derivations());
    }

    #[test]
    fn test_insert_1_derivation() {
        // arrange
        let mut loc = Box::new(MaybeUninit::uninit());
        let tree = unsafe {
            DerivationTree::init_with_root_value(&mut loc, TestNode::new(42));
            assume_init_box(loc)
        };

        // act
        let mut new_node = TestNode::new(42);
        unsafe {
            tree.root_node.insert_derivation(&mut new_node);
        }

        // assert
        assert!(!new_node.tree_data.is_not_in_tree());
        assert_eq!(new_node.tree_data.depth.get(), 2);
        assert!(!new_node.tree_data.prev.get().is_null());
        assert!(new_node.tree_data.next.get().is_null());
        assert!(tree.root_node.has_derivations());
    }

    #[test]
    fn test_drop_node_after_insert_derivation() {
        // arrange
        let mut loc = Box::new(MaybeUninit::uninit());
        let tree = unsafe {
            DerivationTree::init_with_root_value(&mut loc, TestNode::new(42));
            assume_init_box(loc)
        };

        // act
        let mut new_node = TestNode::new(42);
        unsafe {
            tree.root_node.insert_derivation(&mut new_node);
        }
        drop(new_node);

        // assert
        assert!(tree.root_node.tree_data.next.get().is_null());
        assert!(!tree.root_node.has_derivations());
    }

    #[test]
    fn test_drop_node_after_insert_copy() {
        // arrange
        let mut loc = Box::new(MaybeUninit::uninit());
        let tree = unsafe {
            DerivationTree::init_with_root_value(&mut loc, TestNode::new(42));
            assume_init_box(loc)
        };

        // act
        let mut new_node = TestNode::new(42);
        unsafe {
            tree.root_node.insert_copy(&mut new_node);
        }
        drop(new_node);

        // assert
        assert!(tree.root_node.tree_data.next.get().is_null());
        assert!(tree.root_node.is_last_copy());
    }
}
