//! A library for interacting with the lunatix manifest format
//!
//! ## Format
//! The format is basically ini but certain keys and values have special semantics
//!
//! ### Metadata Section
//! The manifest *MAY* contain a `[metadata]` section which defines metadata about the program
//! to which this manifest belongs.
//!
//! This metadata section *MAY* contain any of the following keys:
//! - `name` which is meant as a short identifier for the program.
//! - `description` which should contain a human readable description about what the program does
//!
//! ### Environment Section
//! - `cspace_radix` which denotes the size of the CSpace that holds this programs capabilities.
//!   The cspace is configured to hold `2^cspace_radix` capabilities.
//! - `stack_size` which describes the minimum number of stack bytes that this program needs.
//!
//! ### Capabilities Section
//! The manifest *MAY* contain a `capabilities` section to define which capabilities it expects
//! at which CAddrs when run.
//!
//! When given, this section *MAY* contain one or more capability definitions which must respect
//! the following format:
//! `<caddr>=<type>,<arg1>=<value2>,<arg2>=<value2>,...`
//! This specification dictates that at CAddr `caddr` a capability of the given `type` should be placed.
//! Afterwards `,`-separated arguments *MUST* be given that depend on the capability type.
//!
//! Right now the following capabilities and types are defined:
//! - `cspace` with args:
//!     - `source`: Must be set to `self` to request access to the programs own cspace.
//! - `irq` with args:
//!     - `line`: Must be set to an interrupt to which the irq capability should be bound.
//! - `memory` with args:
//!     - `min_size_bytes`: Must be set to the minimum number of bytes that this memory capability should hold.
#![no_std]

extern crate alloc;

mod capabilities;
mod environment;
mod manifest;
mod metadata;

#[cfg(test)]
mod tests;
