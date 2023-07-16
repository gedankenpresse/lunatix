use crate::caps;

use caps::task::TaskState;
use caps::CSlot;

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

pub fn cspace() -> &'static CSlot {
    let active = active_task();
    return &active.cspace;
}

pub fn vspace() -> &'static CSlot {
    let active = active_task();
    return &active.vspace;
}
