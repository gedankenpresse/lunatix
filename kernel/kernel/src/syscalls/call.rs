use crate::caps::{Capability, Tag};
use crate::sched::Schedule;
use crate::syscalls::handler_trait::SyscallHandler;
use crate::syscalls::ipc::page::page_call;
use crate::syscalls::SyscallContext;
use crate::KernelContext;
use derivation_tree::tree::CursorRefMut;
use syscall_abi::call::Call;
use syscall_abi::{SyscallBinding, SyscallError};

pub(super) struct CallHandler;

impl SyscallHandler for CallHandler {
    type Syscall = Call;

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

        let cap = unsafe {
            cspace
                .resolve_caddr(args.target)
                .unwrap() // TODO Handle error by returning InvalidCaddr
                .as_mut()
                .unwrap()
        };
        log::debug!("dispatching call to {:?} capability", cap.get_tag());
        let result = match cap.get_tag() {
            Tag::Uninit => todo!("call for uninit unimplemented"),
            Tag::Memory => todo!("call for memory unimplemented"),
            Tag::CSpace => todo!("call for cspace unimplemented"),
            Tag::VSpace => todo!("call for vspace unimplemented"),
            Tag::Task => todo!("call to task unimplemented"),
            Tag::Page => page_call(cspace, cap.get_inner_page_mut().unwrap(), args),
            Tag::IrqControl => todo!("call for irq-control unimplemented"),
            Tag::Irq => todo!("call for irq unimplemented"),
            Tag::Notification => todo!("call for notification unimplemented"),
            Tag::Devmem => todo!("call for devmem unimplemented"),
            Tag::AsidControl => todo!("call for asid-control unimplemented"),
            Tag::Endpoint => todo!("call for endpoint unimplemented"),
        };
        (Schedule::Keep, result)
    }
}
