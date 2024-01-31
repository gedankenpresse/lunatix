//! Structure Block handling implementation
//!
//! This implementation is according to [Spec Section 5.4](https://devicetree-specification.readthedocs.io/en/v0.3/flattened-format.html#structure-block)
//!
//! The structure block describes the structure and contents of the devicetree itself.
//! It is composed of a sequence of nodes with properties.
//! These are organized into a linear tree structure.

/// The FDT_BEGIN_NODE token marks the beginning of a node’s representation.
/// It shall be followed by the node’s unit name as extra data.
/// The name is stored as a null-terminated string, and shall include the unit address (see [Spec Section 2.2.1](https://devicetree-specification.readthedocs.io/en/v0.3/devicetree-basics.html#sect-node-names)), if any.
/// The node name is followed by zeroed padding bytes, if necessary for alignment, and then the next token, which may be any token except FDT_END.
const FDT_BEGIN_NODE: u32 = 0x00000001;

/// The FDT_END_NODE token marks the end of a node’s representation.
/// This token has no extra data; so it is followed immediately by the next token, which may be any token except FDT_PROP.
const FDT_END_NODE: u32 = 0x00000002;

/// The FDT_PROP token marks the beginning of the representation of one property in the devicetree.
/// For details see [NodeProperty]().
const FDT_PROP: u32 = 0x00000003;

/// The FDT_NOP token will be ignored by any program parsing the device tree.
/// This token has no extra data; so it is followed immediately by the next token, which can be any valid token.
/// A property or node definition in the tree can be overwritten with FDT_NOP tokens to remove it from the tree without needing to move other sections of the tree’s representation in the devicetree blob.
const FDT_NOP: u32 = 0x00000004;

/// The FDT_END token marks the end of the structure block.
/// There shall be only one FDT_END token, and it shall be the last token in the structure block.
/// It has no extra data; so the byte immediately after the FDT_END token has offset from the beginning of the structure block equal to the value of the [`FdtHeader::size_dt_struct`](super::FdtHeader#structfield.size_dt_struct).
const FDT_END: u32 = 0x00000009;

mod buf_tools;
pub(crate) mod node;
pub(crate) mod property;
