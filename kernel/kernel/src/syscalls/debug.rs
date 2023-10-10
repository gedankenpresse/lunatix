use core::str;
use libkernel::print;
use syscall_abi::debug::{DebugLog, DebugPutc};
use syscall_abi::{NoValue, SyscallBinding};

pub(super) fn sys_debug_log(
    args: <DebugLog as SyscallBinding>::CallArgs,
) -> <DebugLog as SyscallBinding>::Return {
    let str = str::from_utf8(&args.byte_slice[..args.len]).unwrap();
    print!("{}", str);
    Ok(NoValue)
}

pub(super) fn sys_debug_putc(
    args: <DebugPutc as SyscallBinding>::CallArgs,
) -> <DebugPutc as SyscallBinding>::Return {
    print!("{}", args.0);
    Ok(NoValue)
}
