//! Scheduling related functionality and data structures.
//!
//! In detail, this module holds a static variable pointing to the currently active task and provides functions
//! to easily access it.

use crate::caps::{self, Capability};

use caps::task::TaskState;

pub enum Schedule {
    RunInit,
    Keep,
    RunTask(*mut Capability),
    Stop,
}
