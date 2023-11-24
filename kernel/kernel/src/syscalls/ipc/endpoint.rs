use crate::caps::endpoint::{Endpoint, EndpointIface};
use crate::caps::task::TaskExecutionState;
use crate::caps::{Capability, Task};
use crate::sched::Schedule;
use syscall_abi::receive::{Receive, ReceiveReturn};
use syscall_abi::send::SendArgs;
use syscall_abi::{IntoRawSysRepsonse, NoValue, RawSyscallArgs, SyscallBinding, SyscallResult};

fn ipc_recieve_from(src_task: &Task) -> <Receive as SyscallBinding>::Return {
    let src_state = src_task.state.borrow();
    let args: &RawSyscallArgs = src_state.frame.get_syscall_args().try_into().unwrap();
    let SendArgs { tag, raw_args, .. } = SendArgs::from(*args);
    Ok(ReceiveReturn { tag, raw_args })
}

fn wake_endpoint_sender(sender: &Task, result: SyscallResult<NoValue>) {
    log::trace!("waking sender: {:?}", &result);
    let mut state = sender.state.borrow_mut();
    assert!(state.waiting_on.take().is_some());
    state.execution_state = TaskExecutionState::Idle;
    state.frame.write_syscall_return(result.into_response());
}

fn wake_endpoint_receiver(receiver: &Task, result: SyscallResult<ReceiveReturn>) {
    log::trace!("waking receiver: {:?}", &result);
    let mut state = receiver.state.borrow_mut();
    assert!(state.waiting_on.take().is_some());
    state.execution_state = TaskExecutionState::Idle;
    state.frame.write_syscall_return(result.into_response());
}

fn block_endpoint_sender(
    sender: &Task,
    sender_ptr: *mut Capability,
    ep: &Endpoint,
    ep_ptr: *mut Capability,
) {
    log::trace!("blocking endpoint sender");
    unsafe { EndpointIface.add_sender(ep, sender_ptr) };
    let mut task_state = sender.state.borrow_mut();
    assert!(task_state.waiting_on.is_none());
    task_state.waiting_on = Some(ep_ptr);
    task_state.execution_state = TaskExecutionState::Waiting;
}

fn block_endpoint_receiver(
    receiver: &Task,
    receiver_ptr: *mut Capability,
    ep: &Endpoint,
    ep_ptr: *mut Capability,
) {
    log::trace!("blocking endpoint receiver");
    unsafe { EndpointIface.add_receiver(ep, receiver_ptr) };
    let mut task_state = receiver.state.borrow_mut();
    assert!(task_state.waiting_on.is_none());
    task_state.waiting_on = Some(ep_ptr);
    task_state.execution_state = TaskExecutionState::Waiting;
}

pub fn endpoint_send(
    sender_ptr: *mut Capability,
    sender: &Task,
    ep_ptr: *mut Capability,
    ep: &Endpoint,
) -> (Option<SyscallResult<NoValue>>, Schedule) {
    if let Some(x) = ep.state.borrow_mut().recv_set.take() {
        log::trace!("endpoint syncronized, handling send");
        let receiver = unsafe { x.as_mut().unwrap() }.get_inner_task().unwrap();
        let result = ipc_recieve_from(sender);
        wake_endpoint_receiver(receiver, result);
        // TODO: return runTask::destination task
        return (Some(Ok(NoValue)), Schedule::Keep);
    }

    block_endpoint_sender(sender, sender_ptr, ep, ep_ptr);
    (None, Schedule::RunInit)
}

pub fn endpoint_recv(
    receiver_ptr: *mut Capability,
    reciever: &Task,
    ep_ptr: *mut Capability,
    ep: &Endpoint,
) -> (Option<SyscallResult<ReceiveReturn>>, Schedule) {
    if let Some(x) = ep.state.borrow_mut().send_set.take() {
        log::trace!("endpoint syncronized, handling recev");
        let sender = unsafe { x.as_ref().unwrap() }.get_inner_task().unwrap();
        let result = ipc_recieve_from(sender);
        wake_endpoint_sender(sender, Ok(NoValue));
        return (Some(result), Schedule::Keep);
    }

    block_endpoint_receiver(reciever, receiver_ptr, ep, ep_ptr);
    (None, Schedule::RunInit)
}
