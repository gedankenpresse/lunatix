use crate::{
    caps::{CSpaceIface, Capability, Tag},
    syscalls::utils,
};
use derivation_tree::{caps::CapabilityIface, tree::CursorRefMut};
use syscall_abi::{task_assign_cspace::TaskAssignCSpace as Current, NoValue, SyscallBinding};

pub(super) fn sys_task_assign_cspace(
    task: &mut CursorRefMut<'_, '_, Capability>,
    args: <Current as SyscallBinding>::CallArgs,
) -> <Current as SyscallBinding>::Return {
    // get basic caps from task
    let task = task.get_inner_task().unwrap();
    let mut cspace = task.get_cspace();
    let cspace = cspace.get_shared().unwrap();
    let cspace = cspace.get_inner_cspace().unwrap();

    // get valid cspace cap from current tasks cspace
    let source_cspace_cap = unsafe { utils::lookup_cap(cspace, args.cspace_addr, Tag::CSpace) }?;

    // get valid task cap from current tasks cspace
    let target_task_cap = unsafe { utils::lookup_cap_mut(cspace, args.task_addr, Tag::Task) }?;

    // assign cspace to target task
    log::trace!("copy cspace:");
    let task = target_task_cap.get_inner_task_mut().unwrap();
    let mut task = task.state.borrow_mut();
    CSpaceIface.copy(&source_cspace_cap, &mut task.cspace);
    log::trace!("cspace copied");
    Ok(NoValue)
}
