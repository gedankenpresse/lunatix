use crate::caps::task::TaskExecutionState;
use crate::caps::Task;

pub(super) fn sys_exit(task: &Task) {
    let mut task_state = task.state.borrow_mut();
    task_state.execution_state = TaskExecutionState::Exited;
}
