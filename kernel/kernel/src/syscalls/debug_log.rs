use core::str;
use libkernel::print;
use syscall_abi::debug_log::DebugLog as Current;
use syscall_abi::{NoValue, SyscallBinding};

pub(super) fn sys_debug_log(
    args: <Current as SyscallBinding>::CallArgs,
) -> <Current as SyscallBinding>::Return {
    let str = str::from_utf8(&args.byte_slice[..args.len]).unwrap();
    print!("{}", str);
    Ok(NoValue)
}
