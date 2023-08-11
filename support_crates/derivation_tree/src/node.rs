use crate::correspondence::Correspondence;
use crate::cursors::{CursorHandle, CursorSet};
use core::cell::Cell;
use core::mem::ManuallyDrop;
use core::ptr;

/// A TreeNode is an element of a [`DerivationTree`].
pub struct TreeNode<T: Correspondence> {
    // tree collection information
    pub(crate) prev: Cell<*mut TreeNode<T>>,
    pub(crate) next: Cell<*mut TreeNode<T>>,
    pub(crate) depth: Cell<usize>,
    pub(crate) cursors: *const CursorSet<T>, // this could technically be a borrow but that would require a self reference in DerivationTree
    // actual data
    pub value: T,
}

impl<T: Correspondence> TreeNode<T> {
    /// Create a new TreeNode that holds the given value.
    ///
    /// # Safety
    /// Before usage, the TreeNode should be assigned inserted into a collection which must call
    /// [`assign_cursor_set()`](Self::assign_cursor_set).
    pub(crate) unsafe fn new(value: T) -> Self {
        Self {
            prev: Cell::new(ptr::null_mut()),
            next: Cell::new(ptr::null_mut()),
            depth: Cell::new(0),
            cursors: ptr::null(),
            value,
        }
    }

    /// Initialize this node so that it knows about the collections [`CursorSet`].
    ///
    /// # Panics
    /// This function panics when the TreeNode is already part of a [`DerivationTree`] and thus already has a CursorSet
    /// assigned.
    pub(crate) fn assign_cursor_set(&mut self, cursor_set: *mut CursorSet<T>) {
        assert!(
            self.cursors.is_null(),
            "TreeNode already has a CursorSet assigned"
        );
        self.cursors = cursor_set;
    }

    pub(crate) fn get_cursors(&self) -> &CursorSet<T> {
        unsafe { &*self.cursors }
    }

    /// Get a cursor to the last copy of `self`
    pub(crate) fn get_last_copy(&self) -> CursorHandle<T> {
        todo!()
    }

    /// Whether this node is the last copy of the contained value
    pub(crate) fn is_last_copy(&self) -> bool {
        todo!()
    }

    /// Insert the given node into the tree as a sibling of `self`.
    pub(crate) fn insert_after(&self, node: &mut TreeNode<T>) {
        todo!()
    }

    /// Insert the given node into the tree below `self`
    pub(crate) fn insert_below(&self, node: &mut TreeNode<T>) {
        todo!()
    }
}

impl<T: Correspondence> Drop for TreeNode<T> {
    fn drop(&mut self) {
        // ensure that no cursors point to this node
        let self_ptr = self as *mut _;
        assert!(
            !self.get_cursors().exists_cursor_to(self_ptr),
            "TreeNode cannot be safely dropped because there is a cursor pointing to it"
        );

        // remove this node from the linked list of nodes
        unsafe {
            if let Some(prev_node) = self.prev.get().as_ref() {
                prev_node.next.set(self.next.get());
            }
            if let Some(next_node) = self.next.get().as_ref() {
                next_node.prev.set(self.prev.get());
            }
            self.next.set(ptr::null_mut());
            self.prev.set(ptr::null_mut());
            self.depth.set(0);
        }

        // reset cursor pointer to signal that the node is not in a tree
        self.cursors = ptr::null();
    }
}
