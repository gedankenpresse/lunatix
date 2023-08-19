use crate::tree::TreeNodeOps;

/// An iterator over node which walks the chain of `next` pointers.
pub struct NextNodeIterator<T: TreeNodeOps> {
    current_node: Option<*mut T>,
}

impl<T: TreeNodeOps> NextNodeIterator<T> {
    /// Create an iterator which starts at the given node pointer.
    pub fn from_starting_node(node: *mut T) -> Self {
        Self {
            current_node: Some(node),
        }
    }
}

impl<T: TreeNodeOps> Iterator for NextNodeIterator<T> {
    type Item = *mut T;

    fn next(&mut self) -> Option<Self::Item> {
        match self.current_node {
            None => None,
            Some(current_node) => {
                // try to advance internal state to the next node
                let next_node = unsafe { &*current_node }.get_tree_data().next.get();
                if next_node.is_null() {
                    self.current_node = None;
                } else {
                    self.current_node = Some(next_node);
                }

                // return an item
                Some(current_node)
            }
        }
    }
}
