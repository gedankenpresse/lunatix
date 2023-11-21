use derivation_tree::tree::CursorRefMut;
use syscall_abi::destroy::Destroy;
use syscall_abi::{NoValue, SyscallBinding};

use crate::sched::Schedule;
use crate::syscalls::handler_trait::SyscallHandler;
use crate::syscalls::SyscallContext;
use crate::{
    caps::{self},
    KernelContext,
};

pub(super) struct DestroyHandler;

impl SyscallHandler for DestroyHandler {
    type Syscall = Destroy;

    fn handle(
        &mut self,
        _kernel_ctx: &mut KernelContext,
        syscall_ctx: &mut SyscallContext<'_, '_, '_, '_>,
        args: <<Self as SyscallHandler>::Syscall as SyscallBinding>::CallArgs,
    ) -> (
        Schedule,
        <<Self as SyscallHandler>::Syscall as SyscallBinding>::Return,
    ) {
        let task = syscall_ctx.task.get_inner_task().unwrap();
        let mut cspace = task.get_cspace();
        let cspace = cspace.get_shared().unwrap();
        let cspace = cspace.get_inner_cspace().unwrap();

        let target = unsafe {
            cspace
                .resolve_caddr(args.caddr)
                .unwrap() // TODO Handle error by returning InvalidCAddr
                .as_mut()
                .unwrap()
        };

        unsafe { caps::destroy(target) };

        (Schedule::Keep, Ok(NoValue))
    }
}
