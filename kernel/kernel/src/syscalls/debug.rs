use crate::sched::Schedule;
use crate::syscalls::handler_trait::SyscallHandler;
use crate::syscalls::SyscallContext;
use crate::KernelContext;
use core::str;
use klog::print;
use syscall_abi::debug::{DebugLog, DebugPutc};
use syscall_abi::{NoValue, SyscallBinding};

pub(super) struct DebugPutcHandler;

impl SyscallHandler for DebugPutcHandler {
    type Syscall = DebugPutc;

    fn handle(
        &mut self,
        kernel_ctx: &mut KernelContext,
        syscall_ctx: &mut SyscallContext<'_, '_, '_>,
        args: <<Self as SyscallHandler>::Syscall as SyscallBinding>::CallArgs,
    ) -> (
        Schedule,
        <<Self as SyscallHandler>::Syscall as SyscallBinding>::Return,
    ) {
        print!("{}", args.0);
        (Schedule::Keep, Ok(NoValue))
    }
}

pub(super) struct DebugLogHandler;

impl SyscallHandler for DebugLogHandler {
    type Syscall = DebugLog;

    fn handle(
        &mut self,
        kernel_ctx: &mut KernelContext,
        syscall_ctx: &mut SyscallContext<'_, '_, '_>,
        args: <<Self as SyscallHandler>::Syscall as SyscallBinding>::CallArgs,
    ) -> (
        Schedule,
        <<Self as SyscallHandler>::Syscall as SyscallBinding>::Return,
    ) {
        let str = str::from_utf8(&args.byte_slice[..args.len]).unwrap();
        print!("{}", str);
        (Schedule::Keep, Ok(NoValue))
    }
}
