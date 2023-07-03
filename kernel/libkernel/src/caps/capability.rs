//! General capability related structs and utilities

use core::fmt::{Debug, Formatter};

/// A wrapper which holds a capability as well as some general metadata
pub struct CapHolder<C> {
    // TODO Derivation tree, access metadata and other stuff goes in here
    pub cap: C,
}

impl<C> CapHolder<C> {
    pub fn new(cap: C) -> Self {
        Self { cap }
    }
}

impl<C: Debug> Debug for CapHolder<C> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CapHolder").field("cap", &self.cap).finish()
    }
}
