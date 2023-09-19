use derivation_tree::tree::CursorRefMut;
use syscall_abi::task_assign_cspace::{TaskAssignCSpaceArgs, TaskAssignCSpaceReturn};
use crate::caps::{Capability, Tag};

pub(super) fn sys_task_assign_cspace(task: &mut CursorRefMut<'_, '_, Capability>, args: TaskAssignCSpaceArgs) -> TaskAssignCSpaceReturn {
    // get basic caps from task
    let task = task.get_inner_task().unwrap();
    let mut cspace = task.get_cspace();
    let cspace = cspace.get_shared().unwrap();
    let cspace = cspace.get_inner_cspace().unwrap();

    // get valid cspace cap from current tasks cspace
    let target_cspace_cap = match unsafe { cspace.lookup_raw(args.cspace_addr) } {
        None => return TaskAssignCSpaceReturn::InvalidCSpaceAddr,
        Some(cap_ptr) => {
            // TODO Use a cursor to safely access the capability
            let cap = unsafe { &*cap_ptr };
            if *cap.get_tag() != Tag::CSpace {
                return TaskAssignCSpaceReturn::InvalidCSpaceAddr;
            }
            cap
        }
    };

    // get valid task cap from current tasks cspace
    let target_task_cap = match unsafe { cspace.lookup_raw(args.task_addr) } {
        None => return TaskAssignCSpaceReturn::InvalidTaskAddr,
        Some(cap_ptr) => {
            // TODO Use a cursor to safely access the capability
            let cap = unsafe { &mut *cap_ptr };
            if *cap.get_tag() != Tag::Task {
                return TaskAssignCSpaceReturn::InvalidTaskAddr;
            }
            cap
        }
    };

    // assign cspace to target task
    log::trace!("\n\n\n");
    log::trace!("getting cursor to tasks cspace");
    let mut task_cspace = target_task_cap.get_inner_task_mut().unwrap().get_cspace();
    log::trace!("cursor acquired");
    task_cspace.get_exclusive().unwrap();

    todo!()
}