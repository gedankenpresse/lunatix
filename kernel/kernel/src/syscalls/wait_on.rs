use crate::caps::task::TaskExecutionState;
use crate::caps::{Capability, NotificationIface, Tag, TaskIface};
use crate::sched::Schedule;
use crate::syscalls::handler_trait::RawSyscallHandler;
use crate::syscalls::{utils, SyscallContext};
use crate::KernelContext;
use derivation_tree::AsStaticMut;
use syscall_abi::wait_on::{WaitOn, WaitOnArgs};
use syscall_abi::{IntoRawSysRepsonse, NoValue};

pub(super) struct WaitOnHandler;

impl RawSyscallHandler for WaitOnHandler {
    type Syscall = WaitOn;

    fn handle_raw(
        &mut self,
        _kernel_ctx: &mut KernelContext,
        syscall_ctx: &mut SyscallContext<'_, '_>,
    ) -> Schedule {
        // parse arguments
        let raw_args = syscall_ctx.get_raw_args();
        let args = WaitOnArgs::try_from(raw_args).unwrap();

        // get basic caps from task
        let task_cap_ptr = syscall_ctx.task.as_static_mut() as *mut Capability;
        let task = syscall_ctx.task.get_inner_task().unwrap();
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
            task_state.frame.start_pc = syscall_ctx.trap_info.epc;

            Schedule::RunInit
        } else {
            // notification already has a value so we ensure that the task is not blocked anymore and return that value
            unsafe {
                NotificationIface.remove_from_wait_set(notification_cap, task_cap_ptr);
            }
            {
                let mut task_state = task.state.borrow_mut();
                task_state.waiting_on = None;
                task_state.frame.start_pc = syscall_ctx.trap_info.epc + 4;
                task_state
                    .frame
                    .write_syscall_return(Ok(NoValue).into_response())
            }
            TaskIface.wake(&syscall_ctx.task);

            Schedule::Keep
        }
    }
}
