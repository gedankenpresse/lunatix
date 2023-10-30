use crate::sched::Schedule;
use syscall_abi::{r#yield::YieldArgs, NoValue, SyscallResult};

pub(super) fn sys_yield(_args: YieldArgs) -> (SyscallResult<NoValue>, Schedule) {
    (Ok(NoValue), Schedule::RunInit)
}
