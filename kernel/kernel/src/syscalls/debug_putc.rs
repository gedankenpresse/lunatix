use crate::syscalls::SyscallError;
use libkernel::print;
use syscall_abi::debug_putc::{DebugPutcArgs, DebugPutcReturn};

pub(super) fn sys_debug_putc(args: DebugPutcArgs) -> Result<DebugPutcReturn, SyscallError> {
    print!("{}", args.0);
    Ok(DebugPutcReturn::Success)
}
