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

#[test]
fn can_create_cursor() {
    tree!(tree, root = CSlot::new(Capability::Uninit));
    emplace!(mut cursor = tree.root_cursor());
}

#[test]
fn can_dup_cursor() {
    tree!(tree, root = CSlot::new(Capability::Uninit));
    emplace!(mut cursor = tree.root_cursor());
    emplace!(mut dup = cursor.dup());
}

#[test]
fn can_get_mut_ref() {
    tree!(tree, root = CSlot::new(Capability::Uninit));
    emplace!(mut cursor = tree.root_cursor());
    let node = cursor.as_mut().node_mut();
    println!("{:?}", *node);
}

#[test]
fn ref_is_equal_to_root() {
    tree!(tree, root = CSlot::new(Capability::Value(1)));
    emplace!(mut cursor = tree.root_cursor());
    let node = cursor.as_mut().node_mut();
    assert_eq!(&Capability::Value(1), &node.value);
}

#[should_panic]
#[test]
fn cant_create_two_mut_refs() {
    tree!(tree, root = CSlot::new(Capability::Uninit));
    emplace!(mut cursor = tree.root_cursor());
    emplace!(mut dup = cursor.as_ref().dup());
    let ref1 = cursor.as_mut().node_mut();
    let ref2 = dup.as_mut().node_mut();
}

#[test]
fn can_have_to_refs() {
    tree!(tree, root = CSlot::new(Capability::Uninit));
    emplace!(mut cursor = tree.root_cursor());
    emplace!(mut dup = cursor.as_ref().dup());
    let ref1 = cursor.as_mut().node();
    let ref2 = dup.as_mut().node();
}

#[should_panic]
#[test]
fn cant_have_mixed_refs() {
    tree!(tree, root = CSlot::new(Capability::Uninit));
    emplace!(mut cursor = tree.root_cursor());
    emplace!(mut dup = cursor.as_ref().dup());
    let ref1 = cursor.as_mut().node();
    let ref2 = dup.as_mut().node_mut();
}
