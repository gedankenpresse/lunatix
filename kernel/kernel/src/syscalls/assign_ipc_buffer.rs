use crate::caps::{Capability, Tag};
use derivation_tree::tree::CursorRefMut;
use syscall_abi::assign_ipc_buffer::AssignIpcBuffer as Current;
use syscall_abi::NoValue;
use syscall_abi::SyscallBinding;

use super::utils;

pub(super) fn sys_assign_ipc_buffer(
    task: &mut CursorRefMut<'_, '_, Capability>,
    args: <Current as SyscallBinding>::CallArgs,
) -> <Current as SyscallBinding>::Return {
    let task = task.get_inner_task().unwrap();
    let mut cspace = task.get_cspace();
    let cspace = cspace.get_shared().unwrap();
    let cspace = cspace.get_inner_cspace().unwrap();

    let page_cap = unsafe { utils::lookup_cap(cspace, args.page, Tag::Page) }.unwrap();
    let page = page_cap.get_inner_page().unwrap();

    // TODO The mutable ipc_buffer reference we have here is very much unsafe to use (🤷‍)
    let mut taskstate = task.state.borrow_mut();
    taskstate.ipc_buffer = Some(page.kernel_addr);

    Ok(NoValue)
}
