use crate::correspondence::Correspondence;
use crate::cursors::{CursorHandle, CursorSet};
use core::cell::Cell;
use core::ptr;

/// A TreeNode is an element of a [`DerivationTree`](crate::DerivationTree).
///
///
/// # Note on Copies
///
/// This derivation tree does not support distinguishing the concrete node in a set of copies from which another
/// node is derived.
///
/// This means that deriving `m` from an existing node `n` produces a child that is also considered to be a derivation
/// of copies of `n`.
/// The effect is that `m` is only destroyed once the last copy of `n` is destroyed.
///
///
/// # Collection Details
///
/// Since a *DerivationTrees* is an intrinsic collection, the node needs to take part in maintaining it.
/// In detail, the *DerivationTree* is implemented as a double-linked list with an additional depth parameter and
/// specific node ordering.
///
/// ## Ordering Rules
///
/// Nodes are required to be ordered according to the following rules.
/// When considering some *TreeNode* `n`:
/// - Copies of `n` are linked directly `n` and have the same depth value.
/// - Derivations of `n` are linked after the last copy of `n` and have a depth value increased by one.
/// - Siblings of `n` are linked after all derivations of `n`.
///
/// ## Tree Ordering Example
///
/// When considering an example derivation tree like the following, it is internally stored according to the above
/// ordering rules which is also detailed below.
///
/// In this example derivation tree, derivations are presented as a lower tree level and copies are on the same level
/// via `═` connections.
/// This means that `B` and `B'` are copies of each other with `D` and `E` being unrelated to it except that they are
/// derived from the same root node `A` (thus it is a sibling).
/// ```text
///                                        ┌─────────────────────┐
///                                        │  Node A: depth = 1  │
///                                        └──────────┬──────────┘
///                                                   │
///            ┌──────────────────────────────────────┴────────────┬─────────────────────────┐
///            │                                                   │                         │
/// ┌──────────┴──────────┐   ┌─────────────────────┐   ┌──────────┴──────────┐   ┌──────────┴──────────┐
/// │  Node B: depth = 2  ├═══┤ Node B': depth = 2  │   │  Node D: depth = 2  │   │  Node E: depth = 2  │
/// └──────────┬──────────┘   └─────────────────────┘   └─────────────────────┘   └─────────────────────┘
///            │
/// ┌──────────┴──────────┐
/// │  Node C: depth = 3  │
/// └─────────────────────┘
/// ```
///
/// The example tree is internally stored as a double-linked list like this:
/// ```text
/// ┌───────────┐   ┌───────────┐   ┌───────────┐   ┌───────────┐   ┌───────────┐   ┌───────────┐
/// │ Node A    ├───┤ Node B    ├───┤ Node B'   ├───┤ Node C    ├───┤ Node D    ├───┤ Node E    │
/// │ depth = 1 ├───┤ depth = 2 ├───┤ depth = 2 ├───┤ depth = 3 ├───┤ depth = 2 ├───┤ depth = 2 │
/// └───────────┘   └───────────┘   └───────────┘   └───────────┘   └───────────┘   └───────────┘
/// ```
///
/// ## Identifying Copies
///
/// When looking only at the internal double-linked list representation of Derivations trees, it is impossible to
/// distinguish siblings from copies when there are no derivations in between.
/// This can be seen in the above example for nodes `D` and `E`.
///
/// In order to distinguish the two, the trait [`Correspondence`] is used to ask the node if it corresponds to the same
/// thing as another node (which would mean that the two are copies).
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
        // find the last node which corresponds to the same value by walking the next ptr chain
        let mut current_ptr: *mut TreeNode<T> = self as *const _ as *mut _;
        loop {
            let current_node = unsafe { &*current_ptr };
            if let Some(next_node) = unsafe { current_node.next.get().as_ref() } {
                if next_node.value.corresponds_to(&self.value) {
                    assert_eq!(self.depth, next_node.depth);
                    current_ptr = next_node as *const _ as *mut _;
                    continue;
                }
            }

            break;
        }

        // return a cursor pointing to the same node
        let mut cursor = self
            .get_cursors()
            .get_free_cursor()
            .expect("Could not obtain a cursor to point to the last copy");
        cursor.select_node(current_ptr);
        cursor
    }

    /// Whether this node is the last copy of the contained value
    pub(crate) fn is_last_copy(&self) -> bool {
        if let Some(prev_node) = unsafe { self.prev.get().as_ref() } {
            if prev_node.value.corresponds_to(&self.value) {
                return false;
            }
        }

        if let Some(next_node) = unsafe { self.next.get().as_ref() } {
            if next_node.value.corresponds_to(&self.value) {
                return false;
            }
        }

        true
    }

    /// Insert a new node with *copy* ordering.
    ///
    /// Essentially, the new node will be inserted directly after `self` and on the same depth but see the struct
    /// documentation for ordering details.
    ///
    /// Not that this method does not ensure the two nodes are actually copies of each other but instead only inserts
    /// the new node as a copy is supposed to be inserted.
    ///
    /// # Safety
    /// It is unsafe to access the node via its original handle after it has been inserted into the tree.
    /// Instead, a cursor must be obtained from the tree.
    pub(crate) unsafe fn insert_copy(&self, node: &mut TreeNode<T>) {
        assert!(node.is_unlinked());

        let next_ptr = self.next.get();

        // link existing nodes to the new one
        self.next.set(node as *mut _);
        if let Some(next_node) = next_ptr.as_ref() {
            next_node.prev.set(node as *mut _);
        }

        // link the new node to existing ones
        node.prev.set(self as *const _ as *mut _);
        node.next.set(next_ptr);

        // set the same depth
        node.depth.set(self.depth.get());
    }

    /// Insert a new node with *derivation* ordering.
    ///
    /// Essentially, the new node will be inserted after the last copy of `self` and with an increased depth value.
    /// See the struct documentation for ordering details.
    ///
    /// Not that this method does not ensure that `node` actually is a derivation of `self` but instead only inserts
    /// the new node as a derivation is supposed to be inserted.
    ///
    /// # Safety
    /// It is unsafe to access the node via its original handle after it has been inserted into the tree.
    /// Instead, a cursor must be obtained from the tree.
    pub(crate) unsafe fn insert_derivation(&self, node: &mut TreeNode<T>) {
        let mut last_copy_curs = self.get_last_copy();
        let last_copy = last_copy_curs.get_shared().unwrap();

        // this is fine because we are working on the last copy and increase the depth afterwards which makes
        // this a derivation insertion
        last_copy.insert_copy(node);
        node.depth.set(node.depth.get() + 1);
    }

    /// Whether this node has any derivations
    pub(crate) fn has_derivations(&self) -> bool {
        let mut last_copy_curs = self.get_last_copy();
        let last_copy = last_copy_curs.get_shared().unwrap();

        // if the next node has higher depth, it is a derivation of self
        if let Some(next_node) = unsafe { last_copy.next.get().as_ref() } {
            next_node.depth.get() == self.depth.get() + 1
        } else {
            false
        }
    }

    /// Whether this node is currently not linked into any derivation tree
    pub(crate) fn is_unlinked(&self) -> bool {
        self.cursors.is_null()
            && self.depth.get() == 0
            && self.prev.get().is_null()
            && self.next.get().is_null()
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
