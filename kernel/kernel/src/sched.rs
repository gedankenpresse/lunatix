//! Scheduling related functionality and data structures.

use crate::caps::Capability;

#[derive(Debug, Eq, PartialEq)]
pub enum Schedule {
    RunInit,
    Keep,
    RunTask(*mut Capability),
    Stop,
}
