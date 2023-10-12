use crate::caps::{Capability, Tag};
use crate::sched::Schedule;
use derivation_tree::tree::CursorRefMut;
use syscall_abi::yield_to::YieldTo;
use syscall_abi::{NoValue, SyscallBinding};

use super::utils;

pub(super) fn sys_yield_to(
    task: &mut CursorRefMut<'_, '_, Capability>,
    args: <YieldTo as SyscallBinding>::CallArgs,
) -> (<YieldTo as SyscallBinding>::Return, Schedule) {
    // get basic caps from task
    let task = task.get_inner_task().unwrap();
    let mut cspace = task.get_cspace();
    let cspace = cspace.get_shared().unwrap();
    let cspace = cspace.get_inner_cspace().unwrap();

    // get valid memory cap from task
    let target_task_cap = match unsafe { utils::lookup_cap_mut(cspace, args.task, Tag::Task) } {
        Ok(c) => c,
        Err(e) => return (Err(syscall_abi::Error::InvalidCap), Schedule::Keep),
    };

    // TODO Verify that the task is schedulable

    (Ok(NoValue), Schedule::RunTask(target_task_cap as *mut _))
}
