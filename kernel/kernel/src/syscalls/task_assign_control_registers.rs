use crate::caps::{Capability, Tag};
use derivation_tree::tree::CursorRefMut;
use syscall_abi::task_assign_control_registers::{
    TaskAssignControlRegistersArgs, TaskAssignControlRegistersReturn,
};

pub(super) fn sys_task_assign_control_registers(
    task: &mut CursorRefMut<'_, '_, Capability>,
    args: TaskAssignControlRegistersArgs,
) -> TaskAssignControlRegistersReturn {
    // get basic caps from task
    let task = task.get_inner_task().unwrap();
    let mut cspace = task.get_cspace();
    let cspace = cspace.get_shared().unwrap();
    let cspace = cspace.get_inner_cspace().unwrap();

    // get valid task cap from current tasks cspace
    let target_task_cap = match unsafe { cspace.lookup_raw(args.task_addr) } {
        None => return TaskAssignControlRegistersReturn::InvalidTaskAddr,
        Some(cap_ptr) => {
            // TODO Use a cursor to safely access the capability
            let cap = unsafe { &*cap_ptr };
            if *cap.get_tag() != Tag::Task {
                return TaskAssignControlRegistersReturn::InvalidTaskAddr;
            }
            cap
        }
    };

    // TODO Ensure that the task is not currently executing

    // assign control registers as specified by the syscall
    let mut task_state = target_task_cap.get_inner_task().unwrap().state.borrow_mut();
    task_state.frame.start_pc = args.pc;
    task_state.frame.general_purpose_regs[2] = args.sp;
    task_state.frame.general_purpose_regs[3] = args.gp;
    task_state.frame.general_purpose_regs[8] = args.gp;

    TaskAssignControlRegistersReturn::Success
}
