//! Handling of single nodes inside the structure block

use crate::fdt::structure::buf_tools::{align_to_token, ByteSliceWithTokens};
use crate::fdt::structure::property::{NodeProperty, PropertyIter, PropertyParseError};
use crate::fdt::structure::{FDT_BEGIN_NODE, FDT_END_NODE, FDT_PROP};
use crate::fdt::Strings;
use core::ffi::CStr;
use core::mem;
use thiserror_no_std::Error;

#[derive(Debug, Error, Eq, PartialEq)]
pub enum NodeStructureError {
    #[error("The given buffer does not contain a FDT_BEGIN_NODE token at the start")]
    NoNodeBeginToken,
    #[error("The given buffer does not contain a FDT_END_NODE token at the end")]
    NoNodeEndToken,
    #[error("The given buffer does not contain a FDT_END token at the end")]
    NoStructEndToken,
    #[error("The given buffer contained a FDT_BEGIN_NODE token but it was not followed by a string encoding the nodes name")]
    NoNodeName,
    #[error("The given buffer contained a node name but it is invalid UTF-8 even though the spec requires a specific ASCII subset")]
    InvalidNodeName,
    #[error("The name of the root node is not '' as required by the spec")]
    InvalidRootNodeName,
    #[error("The node contained an invalid property: {0}")]
    InvalidProperty(#[from] PropertyParseError),
}

/// A single node inside the structure block.
///
/// Each node consists of the following components:
/// - Node header which contains the node's name (which includes the units memory address if applicable)
/// - Node properties which each contain a name (that is looked up from the strings block) and a value of variable length.
/// - Any number of child nodes which are structured the same.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct StructureNode<'buf> {
    /// The name of the node
    pub name: &'buf str,
    /// The part of the underlying buffer that contains the nodes properties
    props_buf: &'buf [u8],
    /// The part of the underlying buffer that contains this nodes children
    children_buf: &'buf [u8],
    strings: Strings<'buf>,
}

impl<'buf> StructureNode<'buf> {
    /// Interpret a buffer as the root node of the device tree
    pub fn from_buffer_as_root(
        buf: &'buf [u8],
        strings: &Strings<'buf>,
    ) -> Result<Self, NodeStructureError> {
        let (_node_size, node) = Self::from_buffer(buf, strings)?;

        if node.name != "" {
            return Err(NodeStructureError::InvalidRootNodeName);
        }

        Ok(node)
    }

    /// Parse node information from a buffer
    fn from_buffer(
        buf: &'buf [u8],
        strings: &Strings<'buf>,
    ) -> Result<(usize, Self), NodeStructureError> {
        // find the node begin token
        let i_node_begin = buf
            .find_token(FDT_BEGIN_NODE)
            .ok_or(NodeStructureError::NoNodeBeginToken)?;

        // extract node name which follows immediately after FDT_BEGIN_NODE
        let node_name = CStr::from_bytes_until_nul(&buf[i_node_begin + mem::size_of::<u32>()..])
            .map_err(|_| NodeStructureError::NoNodeName)?;
        let node_name_str = node_name
            .to_str()
            .map_err(|_| NodeStructureError::InvalidNodeName)?;

        // parse all properties and record where the last one was parsed
        let i_props_begin = align_to_token(
            i_node_begin + mem::size_of::<u32>() + node_name.to_bytes_with_nul().len(),
        );
        let mut i_props_end = i_props_begin;
        while let Some((offset, FDT_PROP)) = (&buf[i_props_end..]).next_token(true) {
            let (prop_size, prop) =
                NodeProperty::from_buffer(&buf[i_props_end + offset..], strings)?;
            i_props_end = align_to_token(i_props_end + offset + prop_size);
        }

        // parse all child nodes and record where the last one was parsed
        let i_children_begin = align_to_token(i_props_end);
        let mut i_children_end = i_children_begin;
        while let Some((offset, FDT_BEGIN_NODE)) = (&buf[i_children_end..]).next_token(true) {
            let (child_size, _) =
                StructureNode::from_buffer(&buf[i_children_end + offset..], strings)?;
            i_children_end = i_children_end + offset + child_size;
        }

        // assert that there is an FDT_NODE_END token now
        if !matches!(
            (&buf[i_children_end..]).next_token(true),
            Some((0, FDT_END_NODE))
        ) {
            return Err(NodeStructureError::NoNodeEndToken);
        }

        let node = Self {
            name: node_name_str,
            props_buf: &buf[i_props_begin..i_props_end],
            children_buf: &buf[i_children_begin..i_children_end],
            strings: strings.clone(),
        };
        Ok((i_children_end + mem::size_of::<u32>(), node))
    }

    pub fn props(&self) -> PropertyIter<'buf> {
        PropertyIter::new(self.props_buf, self.strings.clone())
    }

    pub fn children(&self) -> NodeIter<'buf> {
        NodeIter::new(self.children_buf, self.strings.clone())
    }
}

/// An iterator over nodes that are encoded in a buffer
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct NodeIter<'buf> {
    pub buf: Option<&'buf [u8]>,
    pub strings: Strings<'buf>,
}

impl<'buf> NodeIter<'buf> {
    pub(super) fn new(buf: &'buf [u8], strings: Strings<'buf>) -> Self {
        Self {
            strings,
            buf: if buf.len() == 0 { None } else { Some(buf) },
        }
    }
}

impl<'buf> Iterator for NodeIter<'buf> {
    type Item = StructureNode<'buf>;

    fn next(&mut self) -> Option<Self::Item> {
        let buf = self.buf?;
        let (node_len, node) = StructureNode::from_buffer(buf, &self.strings).ok()?;
        self.buf = buf.get(align_to_token(node_len)..);
        Some(node)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::fdt::structure::FDT_END;

    #[test]
    fn from_buffer_as_root_works_with_empty_node() {
        let strings = Strings::from_buffer(&[]);
        let mut buf = [0u8; 16];
        buf[0..4].copy_from_slice(&FDT_BEGIN_NODE.to_be_bytes());
        buf[4..8].copy_from_slice(b"\0\0\0\0");
        buf[8..12].copy_from_slice(&FDT_END_NODE.to_be_bytes());
        buf[12..16].copy_from_slice(&FDT_END.to_be_bytes());

        let node = StructureNode::from_buffer_as_root(&buf, &strings).unwrap();
        assert_eq!(node.name, "");
        assert_eq!(node.props().count(), 0);
    }

    #[test]
    fn from_buffer_as_root_works_with_props_only_node() {
        let strings = Strings::from_buffer(b"test\0");
        let mut buf = [0u8; 36];
        buf[0..4].copy_from_slice(&FDT_BEGIN_NODE.to_be_bytes());
        buf[4..8].copy_from_slice(b"\0\0\0\0"); // node name + padding
        buf[8..12].copy_from_slice(&FDT_PROP.to_be_bytes());
        buf[12..16].copy_from_slice(&2u32.to_be_bytes()); // property length = 2 bytes
        buf[16..20].copy_from_slice(&0u32.to_be_bytes()); // property name reference = 0
        buf[20..22].copy_from_slice(&[0xff, 0xff]); // property value
        buf[22..24].copy_from_slice(&[0, 0]); // padding bytes
        buf[24..28].copy_from_slice(&FDT_END_NODE.to_be_bytes());
        buf[28..32].copy_from_slice(&FDT_END.to_be_bytes());

        let node = StructureNode::from_buffer_as_root(&buf, &strings).unwrap();
        assert_eq!(node.name, "");
        assert_eq!(node.props().count(), 1);
        assert_eq!(node.props().nth(0).unwrap().name, "test");
        assert_eq!(node.props().nth(0).unwrap().value, &[0xff, 0xff]);
    }

    #[test]
    fn from_buffer_as_root_works_with_children() {
        let strings = Strings::from_buffer(b"");
        let mut buf = [0u8; 64];
        buf[0..4].copy_from_slice(&FDT_BEGIN_NODE.to_be_bytes()); // root node start
        buf[4..8].copy_from_slice(b"\0\0\0\0"); // name + padding
        buf[8..12].copy_from_slice(&FDT_BEGIN_NODE.to_be_bytes()); // child node
        buf[12..20].copy_from_slice(b"child\0\0\0"); // child name + padding
        buf[20..24].copy_from_slice(&FDT_END_NODE.to_be_bytes()); // child node end
        buf[24..28].copy_from_slice(&FDT_END_NODE.to_be_bytes()); // root node end
        buf[28..32].copy_from_slice(&FDT_END.to_be_bytes()); // block end

        let node = StructureNode::from_buffer_as_root(&buf, &strings).unwrap();
        assert_eq!(node.children().count(), 1);
        assert_eq!(node.children().nth(0).unwrap().name, "child");
    }
}
