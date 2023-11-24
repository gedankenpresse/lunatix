use derivation_tree::tree::CursorRefMut;
use syscall_abi::copy::Copy;
use syscall_abi::{NoValue, SyscallBinding};

use crate::sched::Schedule;
use crate::syscalls::handler_trait::SyscallHandler;
use crate::syscalls::SyscallContext;
use crate::{
    caps::{self, Capability, SyscallError},
    KernelContext,
};

pub(super) struct CopyHandler;

impl SyscallHandler for CopyHandler {
    type Syscall = Copy;

    fn handle(
        &mut self,
        kernel_ctx: &mut KernelContext,
        syscall_ctx: &mut SyscallContext<'_, '_>,
        args: <<Self as SyscallHandler>::Syscall as SyscallBinding>::CallArgs,
    ) -> (
        Schedule,
        <<Self as SyscallHandler>::Syscall as SyscallBinding>::Return,
    ) {
        let task = syscall_ctx.task.get_inner_task().unwrap();
        let mut cspace = task.get_cspace();
        let cspace = cspace.get_shared().unwrap();
        let cspace = cspace.get_inner_cspace().unwrap();

        let src = unsafe {
            cspace
                .resolve_caddr(args.src)
                .unwrap() // TODO handle error by returning InvalidCAddr
                .as_ref()
                .unwrap()
        };
        let target = unsafe {
            cspace
                .resolve_caddr(args.dst)
                .unwrap() // TODO handle error by returning InvalidCAddr
                .as_mut()
                .unwrap()
        };

        unsafe { caps::copy(src, target) };

        (Schedule::Keep, Ok(NoValue))
    }
}
