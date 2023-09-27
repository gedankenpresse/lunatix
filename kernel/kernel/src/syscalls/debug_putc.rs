use libkernel::print;
use syscall_abi::{debug_putc::DebugPutc as Current, NoValue, SyscallBinding};

pub(super) fn sys_debug_putc(
    args: <Current as SyscallBinding>::CallArgs,
) -> <Current as SyscallBinding>::Return {
    print!("{}", args.0);
    Ok(NoValue)
}
