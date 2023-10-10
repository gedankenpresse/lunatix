use crate::sched::Schedule;
use syscall_abi::{r#yield::YieldArgs, NoValue, SyscallResult};

pub(super) fn sys_yield(args: YieldArgs) -> (SyscallResult<NoValue>, Schedule) {
    (Ok(NoValue), Schedule::RunInit)
}
