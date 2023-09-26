use crate::sched::Schedule;
use syscall_abi::r#yield::{YieldArgs, YieldReturn};

pub(super) fn sys_yield(args: YieldArgs) -> (YieldReturn, Schedule) {
    (YieldReturn::Success, Schedule::RunInit)
}
