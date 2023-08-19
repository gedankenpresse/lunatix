use crate::caps::{self, Capability};

use caps::task::TaskState;

static mut ACTIVE_TASK: *mut TaskState = core::ptr::null_mut();

pub unsafe fn set_active_task(state: *mut TaskState) {
    ACTIVE_TASK = state;
}

#[inline(always)]
fn active_task() -> &'static mut TaskState {
    unsafe {
        let active = ACTIVE_TASK.as_mut().expect("No ACTIVE_TASK found");
        return active;
    }
}

pub fn cspace() -> &'static Capability {
    let active = active_task();
    return &active.cspace;
}

pub fn vspace() -> &'static Capability {
    let active = active_task();
    return &active.vspace;
}
