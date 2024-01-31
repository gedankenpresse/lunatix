//! Device-Tree-Blob (also called flattened device tree) handling
//!
//! The DTB format encodes the devicetree data within a single, linear, pointerless data structure.
//! It consists of a small header (see [Spec Section 5.2](https://devicetree-specification.readthedocs.io/en/latest/chapter5-flattened-format.html#sect-fdt-structure-block)),
//! followed by three variable sized sections:
//!
//! - the memory reservation block (see [Spec Section 5.3](https://devicetree-specification.readthedocs.io/en/latest/chapter5-flattened-format.html#sect-fdt-memory-reservation-block)),
//! - the structure block (see [Spec Section 5.4](https://devicetree-specification.readthedocs.io/en/latest/chapter5-flattened-format.html#sect-fdt-structure-block)),
//! - and the strings block (see [Spec Section 5.5](https://devicetree-specification.readthedocs.io/en/latest/chapter5-flattened-format.html#sect-fdt-strings-block)).
//!
//! These should be present in the flattened devicetree in that order. Thus, the devicetree structure as a whole, when loaded into memory at address, will resemble the below diagram (lower addresses are at the top of the diagram).
//! The `(free space)` sections *may* not be present, though in some cases they might be required to satisfy the alignment constraints of the individual blocks
//! ```text
//! ┌──────────────────────────┐
//! │ struct FdtHeader         │
//! ├──────────────────────────┤
//! │ (free space)             │
//! ├──────────────────────────┤
//! │ memory reservation block │
//! ├──────────────────────────┤
//! │ (free space)             │
//! ├──────────────────────────┤
//! │ structure block          │
//! ├──────────────────────────┤
//! │ (free space)             │
//! ├──────────────────────────┤
//! │ strings block            │
//! └──────────────────────────┘
//! ```

mod header;
mod memory_reservation;
mod strings;
mod structure;

pub use header::{FdtHeader, HeaderReadError};
pub use memory_reservation::{FormatError, MemoryReservationBlock, MemoryReservationEntry};
pub use strings::{Strings, StringsError};
