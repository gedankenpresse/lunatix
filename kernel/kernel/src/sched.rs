//! Scheduling related functionality and data structures.

use crate::caps::Capability;

pub enum Schedule {
    RunInit,
    Keep,
    RunTask(*mut Capability),
    Stop,
}
