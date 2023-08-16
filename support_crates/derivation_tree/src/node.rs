use crate::cursors::{CursorData, CursorSet};
use crate::Correspondence;
use core::cell::Cell;
use core::ptr;

/// A TreeNode is an element of a [`DerivationTree`](crate::DerivationTree) and this trait must be implemented by all
/// types that should be stored in one.
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
pub trait TreeNodeOps: Sized + Correspondence {
    /// Return the data structure which holds all tree-related information
    fn get_tree_data(&self) -> &TreeNodeData<Self>;

    /// Get a cursor to the last copy of `self`
    ///
    /// # Safety
    /// You are not allowed to drop any node while holding the returned pointer.
    unsafe fn get_last_copy(&self) -> *mut Self {
        let tree_data = self.get_tree_data();

        // find the last node which corresponds to the same value by walking the next ptr chain
        let mut current_ptr: *mut Self = self as *const _ as *mut _;
        loop {
            if let Some(next_node) = unsafe { tree_data.next.get().as_ref() } {
                if next_node.corresponds_to(&self) {
                    assert_eq!(self.get_tree_data().depth, next_node.get_tree_data().depth);
                    current_ptr = next_node as *const _ as *mut _;
                    continue;
                }
            }

            break;
        }

        current_ptr
    }

    /// Whether this node is the last copy of the contained value
    fn is_last_copy(&self) -> bool {
        let tree_data = self.get_tree_data();

        if let Some(prev_node) = unsafe { tree_data.prev.get().as_ref() } {
            if prev_node.corresponds_to(&self) {
                return false;
            }
        }

        if let Some(next_node) = unsafe { self.get_tree_data().next.get().as_ref() } {
            if next_node.corresponds_to(&self) {
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
    unsafe fn insert_copy(&self, node: &mut Self) {
        assert!(node.get_tree_data().is_unlinked());

        let self_tree_data = self.get_tree_data();
        let next_ptr = self.get_tree_data().next.get();

        // link existing nodes to the new one
        self_tree_data.next.set(node as *mut _);
        if let Some(next_node) = next_ptr.as_ref() {
            next_node.get_tree_data().prev.set(node as *mut _);
        }

        // link the new node to existing ones
        let node_tree_data = node.get_tree_data();
        node_tree_data.prev.set(self as *const _ as *mut _);
        node_tree_data.next.set(next_ptr);

        // set the same depth
        node_tree_data.depth.set(self_tree_data.depth.get());

        // give that node access to the trees cursors
        node_tree_data.cursors.set(self_tree_data.cursors.get())
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
    unsafe fn insert_derivation(&self, node: &mut Self) {
        let last_copy = &mut *self.get_last_copy();

        // this is fine because we are working on the last copy and increase the depth afterwards which makes
        // this a derivation insertion
        last_copy.insert_copy(node);
        node.get_tree_data()
            .depth
            .set(node.get_tree_data().depth.get() + 1);
    }

    /// Whether this node has any derivations
    fn has_derivations(&self) -> bool {
        let last_copy = unsafe { &mut *self.get_last_copy() };

        // if the next node has higher depth, it is a derivation of self
        if let Some(next_node) = unsafe { last_copy.get_tree_data().next.get().as_ref() } {
            next_node.get_tree_data().depth.get() == self.get_tree_data().depth.get() + 1
        } else {
            false
        }
    }
}

/// A data structure to hold all information necessary for tracking an element in a [`DerivationTree`](crate::DerivationTree).
pub struct TreeNodeData<T: TreeNodeOps> {
    /// A pointer to the previous tree element.
    ///
    /// See the [`TreeNodeOps`] documentation for information about the internally used linked list.
    pub(crate) prev: Cell<*mut T>,

    /// A pointer to the next tree element.
    ///
    /// See the [`TreeNodeOps`] documentation for information about the internally used linked list.
    pub(crate) next: Cell<*mut T>,

    /// A depth measurement used to record derivation information.
    ///
    /// See the [`TreeNodeOps`] documentation for information about the linked-list ordering.
    pub(crate) depth: Cell<usize>,

    /// A pointer to a *CursorSet* that can be used to safely access the tree.
    pub(crate) cursors: Cell<*const CursorSet<T>>, // this could technically be a borrow but that would require a self reference in DerivationTree
}

impl<T: TreeNodeOps> TreeNodeData<T> {
    /// Create a new unlinked instance.
    ///
    /// # Safety
    /// Before usage, the containing TreeNode should be assigned inserted into a collection which must call
    /// [`assign_cursor_set()`](Self::assign_cursor_set).
    pub(crate) unsafe fn new() -> Self {
        Self {
            prev: Cell::new(ptr::null_mut()),
            next: Cell::new(ptr::null_mut()),
            depth: Cell::new(0),
            cursors: Cell::new(ptr::null()),
        }
    }

    /// Initialize this node so that it knows about the collections [`CursorSet`].
    ///
    /// # Panics
    /// This function panics when the TreeNode is already part of a [`DerivationTree`] and thus already has a CursorSet
    /// assigned.
    pub fn assign_cursor_set(&self, cursor_set: *mut CursorSet<T>) {
        assert!(
            self.cursors.get().is_null(),
            "TreeNode already has a CursorSet assigned"
        );
        self.cursors.set(cursor_set);
    }

    pub fn get_cursors(&self) -> &CursorSet<T> {
        unsafe { &*self.cursors.get() }
    }

    /// Whether this node is currently not linked into any derivation tree
    pub fn is_unlinked(&self) -> bool {
        self.cursors.get().is_null()
            && self.depth.get() == 0
            && self.prev.get().is_null()
            && self.next.get().is_null()
    }
}

impl<T: TreeNodeOps> Drop for TreeNodeData<T> {
    fn drop(&mut self) {
        // ensure that no cursors point to this node
        let self_ptr = self as *const _;
        assert!(
            !self
                .get_cursors()
                .cursor_iter()
                .any(|cursor| match cursor.get() {
                    CursorData::Free => false,
                    CursorData::Allocated => false,
                    CursorData::Inactive(node_ptr) =>
                        unsafe { &*node_ptr }.get_tree_data() as *const _ == self_ptr,
                    CursorData::SharedRef(node_ptr) =>
                        unsafe { &*node_ptr }.get_tree_data() as *const _ == self_ptr,
                    CursorData::ExclusiveRef(node_ptr) =>
                        unsafe { &*node_ptr }.get_tree_data() as *const _ == self_ptr,
                }),
            "TreeNode cannot be safely dropped because there is a cursor pointing to it"
        );

        // remove this node from the linked list of nodes
        unsafe {
            if let Some(prev_node) = self.prev.get().as_ref() {
                prev_node.get_tree_data().next.set(self.next.get());
            }
            if let Some(next_node) = self.next.get().as_ref() {
                next_node.get_tree_data().prev.set(self.prev.get());
            }
            self.next.set(ptr::null_mut());
            self.prev.set(ptr::null_mut());
            self.depth.set(0);
        }

        // reset cursor pointer to signal that the node is not in a tree
        self.cursors.set(ptr::null());
    }
}
