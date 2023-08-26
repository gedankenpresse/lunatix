use crate::caps::Tag;
use crate::sched;
use syscall_abi::assign_ipc_buffer::{AssignIpcBufferArgs, AssignIpcBufferReturn};

pub(super) fn sys_assign_ipc_buffer(args: AssignIpcBufferArgs) -> AssignIpcBufferReturn {
    let cspace = sched::cspace().get_inner_cspace().unwrap();

    let page_cap = match unsafe { cspace.lookup_raw(args.page) } {
        None => return AssignIpcBufferReturn::Error,
        Some(cap_ptr) => {
            // TODO Use a cursor to safely access the capability
            let cap = unsafe { &*cap_ptr };
            if *cap.get_tag() != Tag::Page {
                return AssignIpcBufferReturn::Error;
            }
            cap
        }
    };
    let page = page_cap.get_inner_page().unwrap();

    // TODO The mutable ipc_buffer reference we have here is very much unsafe to use (🤷‍)
    sched::get_active_task().ipc_buffer = Some(page.kernel_addr);

    AssignIpcBufferReturn::Success
}
