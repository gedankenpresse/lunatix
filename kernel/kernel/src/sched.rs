use crate::caps::{task::TaskState, Node, Capability};

static mut ACTIVE_TASK: *mut TaskState = core::ptr::null_mut();

#[inline(always)]
fn active_task() -> &'static mut TaskState {
    unsafe { 
        let active = ACTIVE_TASK.as_mut().expect("No ACTIVE_TASK found");
        return active;
    }
}

pub fn cspace() -> &'static mut Node<Capability> {
    let active = active_task();
    let cap = &mut active.cspace.cap;
    return cap;
}

pub fn vspace() -> &'static mut Node<Capability> {
    let active = active_task();
    let cap = &mut active.vspace.cap;
    return cap;
}