use crate::caps::{Capability, Tag};
use crate::sched::Schedule;
use derivation_tree::tree::CursorRefMut;
use syscall_abi::yield_to::{YieldToArgs, YieldToReturn};

pub(super) fn sys_yield_to(
    task: &mut CursorRefMut<'_, '_, Capability>,
    args: YieldToArgs,
) -> (YieldToReturn, Schedule) {
    // get basic caps from task
    let task = task.get_inner_task().unwrap();
    let mut cspace = task.get_cspace();
    let cspace = cspace.get_shared().unwrap();
    let cspace = cspace.get_inner_cspace().unwrap();

    // get valid memory cap from task
    let target_task_cap = match unsafe { cspace.lookup_raw(args.task) } {
        None => return (YieldToReturn::InvalidTaskAddr, Schedule::Keep),
        Some(cap_ptr) => {
            // TODO Use a cursor to safely access the capability
            let cap = unsafe { &mut *cap_ptr };
            if *cap.get_tag() != Tag::Task {
                return (YieldToReturn::InvalidTaskAddr, Schedule::Keep);
            }
            cap
        }
    };

    // TODO Verify that the task is schedulable

    (
        YieldToReturn::Success,
        Schedule::RunTask(target_task_cap as *mut _),
    )
}
