extern crate std;
use std::pin::{pin, Pin};

use ctors::emplace;
use derivation_tree::{tree, tree_node::*};

#[derive(Debug, PartialEq, Eq)]
enum Capability {
    Uninit,
    Value(usize),
}

type CSlot = TreeNode<Capability>;
// type CapTree = CursorSet<CSlot>;

#[test]
fn can_build_cap_tree() {
    tree!(tree, root = CSlot::new(Capability::Value(1)));
    tree.uninit_cursor();
}

#[should_panic]
#[test]
fn can_build_cap_tree_panic() {
    tree!(tree, root = CSlot::new(Capability::Uninit));
}

#[test]
fn can_create_cursor() {
    tree!(tree, root = CSlot::new(Capability::Uninit));
    {
        emplace!(mut cursor = tree.as_mut().root_cursor());
    }
    tree.uninit_cursor();
}

/*
#[test]
#[should_panic]
fn uninit_panics_with_active_cursors() {
    tree!(tree, root = CSlot::new(Capability::Uninit));
    emplace!(mut cursor = tree.as_mut().root_cursor());
    tree.uninit_cursor();
}
*/
