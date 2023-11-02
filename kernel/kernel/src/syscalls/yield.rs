use crate::sched::Schedule;
use syscall_abi::r#yield::Yield;
use syscall_abi::{NoValue, SyscallBinding};

pub(super) fn sys_yield(
    _args: <Yield as SyscallBinding>::CallArgs,
) -> (<Yield as SyscallBinding>::Return, Schedule) {
    (Ok(NoValue), Schedule::RunInit)
}
