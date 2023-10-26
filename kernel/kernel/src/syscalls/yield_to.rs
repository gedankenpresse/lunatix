use crate::caps::task::{TaskExecutionState, TaskState};
use crate::caps::{Capability, Tag};
use crate::sched::Schedule;
use derivation_tree::tree::CursorRefMut;
use syscall_abi::yield_to::{TaskStatus, YieldTo};
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
    let target_task_ptr = target_task_cap as *mut Capability;
    let target_task = target_task_cap.get_inner_task().unwrap();
    let target_task_State = target_task.state.borrow();

    match target_task_State.execution_state {
        TaskExecutionState::Running => (Ok(TaskStatus::AlreadyRunning), Schedule::Keep),
        TaskExecutionState::Waiting => (Ok(TaskStatus::Blocked), Schedule::Keep),
        TaskExecutionState::Idle => (
            Ok(TaskStatus::DidExecute),
            Schedule::RunTask(target_task_ptr),
        ),
        TaskExecutionState::Exited => (Ok(TaskStatus::Exited), Schedule::Keep),
    }
}
