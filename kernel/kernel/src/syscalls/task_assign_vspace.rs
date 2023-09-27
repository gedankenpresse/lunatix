use crate::{
    caps::{Capability, Tag, VSpaceIface},
    syscalls::utils,
};
use derivation_tree::caps::CapabilityIface;
use derivation_tree::tree::CursorRefMut;
use syscall_abi::{task_assign_vspace::TaskAssignVSpace as Current, NoValue, SyscallBinding};

pub(super) fn sys_task_assign_vspace(
    task: &mut CursorRefMut<'_, '_, Capability>,
    args: <Current as SyscallBinding>::CallArgs,
) -> <Current as SyscallBinding>::Return {
    // get basic caps from task
    let task = task.get_inner_task().unwrap();
    let mut cspace = task.get_cspace();
    let cspace = cspace.get_shared().unwrap();
    let cspace = cspace.get_inner_cspace().unwrap();

    // get valid cspace cap from current tasks cspace
    let source_vspace_cap = unsafe { utils::lookup_cap(cspace, args.vspace_addr, Tag::VSpace) }?;

    // get valid task cap from current tasks cspace
    let target_task_cap = unsafe { utils::lookup_cap_mut(cspace, args.task_addr, Tag::Task) }?;

    // assign cspace to target task
    log::trace!("copy vspace:");
    let task = target_task_cap.get_inner_task_mut().unwrap();
    let mut task = task.state.borrow_mut();
    VSpaceIface.copy(&source_vspace_cap, &mut task.vspace);
    log::trace!("vspace copied");
    Ok(NoValue)
}
