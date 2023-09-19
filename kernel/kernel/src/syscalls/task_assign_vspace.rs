use derivation_tree::tree::CursorRefMut;
use syscall_abi::task_assign_cspace::{TaskAssignCSpaceArgs, TaskAssignCSpaceReturn};
use syscall_abi::task_assign_vspace::{TaskAssignVSpaceArgs, TaskAssignVSpaceReturn};
use crate::caps::{Capability, Tag};

pub(super) fn sys_task_assign_vspace(task: &mut CursorRefMut<'_, '_, Capability>, args: TaskAssignVSpaceArgs) -> TaskAssignVSpaceReturn {
    // get basic caps from task
    let task = task.get_inner_task().unwrap();
    let mut cspace = task.get_cspace();
    let cspace = cspace.get_shared().unwrap();
    let cspace = cspace.get_inner_cspace().unwrap();

    // get valid cspace cap from current tasks cspace
    let target_cspace_cap = match unsafe { cspace.lookup_raw(args.vspace_addr) } {
        None => return TaskAssignVSpaceReturn::InvalidVSpaceAddr,
        Some(cap_ptr) => {
            // TODO Use a cursor to safely access the capability
            let cap = unsafe { &*cap_ptr };
            if *cap.get_tag() != Tag::VSpace {
                return TaskAssignVSpaceReturn::InvalidVSpaceAddr;
            }
            cap
        }
    };

    // get valid task cap from current tasks cspace
    let target_task_cap = match unsafe { cspace.lookup_raw(args.task_addr) } {
        None => return TaskAssignVSpaceReturn::InvalidTaskAddr,
        Some(cap_ptr) => {
            // TODO Use a cursor to safely access the capability
            let cap = unsafe { &*cap_ptr };
            if *cap.get_tag() != Tag::Task {
                return TaskAssignVSpaceReturn::InvalidTaskAddr;
            }
            cap
        }
    };

    todo!()
}