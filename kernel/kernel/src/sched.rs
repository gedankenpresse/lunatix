//! Scheduling related functionality and data structures.
//!
//! In detail, this module holds a static variable pointing to the currently active task and provides functions
//! to easily access it.

use crate::caps::{self, Capability};

use caps::task::TaskState;

static mut ACTIVE_TASK: *mut TaskState = core::ptr::null_mut();

pub unsafe fn set_active_task(state: *mut TaskState) {
    ACTIVE_TASK = state;
}

#[inline(always)]
pub fn get_active_task() -> &'static mut TaskState {
    unsafe {
        let active = ACTIVE_TASK.as_mut().expect("No ACTIVE_TASK found");
        return active;
    }
}

pub fn cspace() -> &'static Capability {
    let active = get_active_task();
    return &active.cspace;
}

pub fn vspace() -> &'static Capability {
    let active = get_active_task();
    return &active.vspace;
}

pub enum Schedule {
    RunInit,
    Keep,
    RunTask(*mut Capability),
    Stop,
}
