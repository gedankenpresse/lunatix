use crate::caps::task::TaskExecutionState;
use crate::caps::{Capability, NotificationIface, Tag, TaskIface};
use crate::sched::Schedule;
use crate::syscalls::utils;
use derivation_tree::tree::CursorRefMut;
use derivation_tree::AsStaticMut;
use syscall_abi::wait_on::WaitOnArgs;
use syscall_abi::{Error, SyscallResult};

pub(super) fn sys_wait_on(
    task_cap: &mut CursorRefMut<'_, '_, Capability>,
    args: WaitOnArgs,
) -> (SyscallResult<usize>, Schedule) {
    // get basic caps from task
    let task_cap_ptr = task_cap.as_static_mut() as *mut Capability;
    let task = task_cap.get_inner_task().unwrap();
    let mut cspace = task.get_cspace();
    let cspace = cspace.get_shared().unwrap();
    let cspace = cspace.get_inner_cspace().unwrap();

    // get valid notification from cspace
    let notification_cap =
        unsafe { utils::lookup_cap(cspace, args.notification, Tag::Notification) }.unwrap();

    let value = NotificationIface.take_value(notification_cap);
    if value == 0 {
        // notification did not contain anything so the task needs to be blocked
        unsafe {
            NotificationIface.add_to_wait_set(notification_cap, task_cap_ptr);
        }
        let mut task_state = task.state.borrow_mut();
        task_state.execution_state = TaskExecutionState::Waiting;
        task_state.waiting_on = Some(notification_cap as *const Capability);

        // bump down program counter by one instruction so that the same syscall is executed again once the
        // notification triggers.
        // this has the effect of then executing the branch below to construct a valid return value
        task_state.frame.start_pc -= 4;

        (Err(Error::WouldBlock), Schedule::RunInit)
    } else {
        // notification already has a value so we ensure that the task is not blocked anymore and return that value
        unsafe {
            NotificationIface.remove_from_wait_set(notification_cap, task_cap_ptr);
        }
        {
            let mut task_state = task.state.borrow_mut();
            task_state.waiting_on = None;
        }
        TaskIface.wake(task_cap);

        (Ok(value), Schedule::Keep)
    }
}
