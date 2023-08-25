use core::str;
use libkernel::print;
use syscall_abi::debug_log::{DebugLog, DebugLogReturn};
use syscall_abi::SyscallBinding;

pub(super) fn sys_debug_log(
    args: <DebugLog as SyscallBinding>::CallArgs,
) -> Result<<DebugLog as SyscallBinding>::Return, ()> {
    let str = str::from_utf8(&args.byte_slice[..args.len]).unwrap();
    print!("{}", str);
    Ok(DebugLogReturn::Success)
}
