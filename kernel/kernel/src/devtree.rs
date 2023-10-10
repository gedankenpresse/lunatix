use fdt_rs::{
    base::{DevTree, DevTreeNode},
    prelude::*,
};

use crate::caps::DevmemEntry;

pub struct ExternalDevice<'buf, 'dt> {
    node: DevTreeNode<'buf, 'dt>,
}

pub struct ExternalDeviceInfo<'buf> {
    name: &'buf str,
    compatible: &'buf str,
    reg_base: u64,
    reg_size: u64,
}

fn has_props<'a, 'dt>(node: &DevTreeNode<'a, 'dt>, props: &[&str]) -> bool {
    'outer: for &prop in props {
        let mut node_props = node.props();
        while let Ok(Some(node_prop)) = node_props.next() {
            if node_prop.name() == Ok(prop) {
                continue 'outer;
            }
        }
        return false;
    }
    true
}

fn nodes_with_props<'a, 'p: 'a, 'dt>(
    fdt: &'a DevTree<'dt>,
    props: &'p [&'static str],
) -> impl Iterator<Item = DevTreeNode<'a, 'dt>> + 'a {
    let mut dev_tree_nodes = fdt.nodes();
    core::iter::from_fn(move || {
        while let Ok(Some(node)) = dev_tree_nodes.next() {
            if has_props(&node, props) {
                return Some(node);
            }
        }
        None
    })
}

pub fn read_reg_prop<'a, 'dt>(node: &DevTreeNode<'a, 'dt>) -> Option<DevmemEntry> {
    let mut props = node.props();
    while let Ok(Some(prop)) = props.next() {
        if prop.name() == Ok("reg") {
            let base = prop.u64(0);
            let len = prop.u64(1);
            let entry = DevmemEntry {
                base: base.ok()? as usize,
                len: len.ok()? as usize,
            };
            return Some(entry);
        }
    }
    None
}

pub fn get_external_devices<'buf, 'a, 'dt>(
    fdt: &'a DevTree<'dt>,
    buf: &'buf mut [Option<DevmemEntry>],
) -> &'buf [Option<DevmemEntry>] {
    let mut pos = 0;
    for (i, (node, slot)) in nodes_with_props(fdt, &["reg", "compatible", "interrupt-parent"])
        .zip(buf.iter_mut())
        .enumerate()
    {
        log::info!("external device found: {}", node.name().unwrap());
        *slot = read_reg_prop(&node);
        pos = i;
    }
    &buf[0..pos]
}
