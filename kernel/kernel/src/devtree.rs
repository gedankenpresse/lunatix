use fdt_rs::{
    base::{DevTree, DevTreeNode},
    prelude::*,
};
use libkernel::println;

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

pub fn get_external_devices<'buf, 'a, 'dt>(
    fdt: &'a DevTree<'dt>,
    buf: &'buf mut [Option<ExternalDevice<'a, 'dt>>],
) -> &'buf [Option<ExternalDevice<'a, 'dt>>] {
    let mut pos = 0;
    for (i, (node, slot)) in nodes_with_props(fdt, &["reg", "compatible", "interrupt-parent"])
        .zip(buf.iter_mut())
        .enumerate()
    {
        println!("{}", node.name().unwrap());
        *slot = Some(ExternalDevice { node });
        pos = i;
    }
    &buf[0..pos]
}
