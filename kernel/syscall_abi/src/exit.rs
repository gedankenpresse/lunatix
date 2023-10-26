use crate::{NoValue, RawSyscallArgs, SyscallBinding, SyscallResult};

#[derive(Debug)]
pub struct Exit;

impl SyscallBinding for Exit {
    const SYSCALL_NO: usize = 22;
    type CallArgs = RawSyscallArgs;
    type Return = SyscallResult<NoValue>;
}
