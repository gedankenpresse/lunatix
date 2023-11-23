use crate::caps::{Capability, Tag};
use crate::sched::Schedule;
use crate::syscalls::handler_trait::SyscallHandler;
use crate::syscalls::SyscallContext;
use crate::KernelContext;
use syscall_abi::identify::Identify;
use syscall_abi::{identify::CapabilityVariant, SyscallBinding, SyscallError};

pub(super) struct IdentifyHandler;

impl SyscallHandler for IdentifyHandler {
    type Syscall = Identify;

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

        let cap_ptr = match unsafe { cspace.resolve_caddr(args.caddr) } {
            Some(ptr) => ptr,
            None => return (Schedule::Keep, Err(SyscallError::InvalidCAddr)),
        };

        // TODO Use a cursor to safely access the capability
        let cap = unsafe { &*cap_ptr };
        let tag = cap.get_tag();
        let variant = match tag {
            Tag::Uninit => CapabilityVariant::Uninit,
            Tag::Memory => CapabilityVariant::Memory,
            Tag::CSpace => CapabilityVariant::CSpace,
            Tag::VSpace => CapabilityVariant::VSpace,
            Tag::Task => CapabilityVariant::Task,
            Tag::Page => CapabilityVariant::Page,
            Tag::IrqControl => CapabilityVariant::IrqControl,
            Tag::Irq => CapabilityVariant::Irq,
            Tag::Notification => CapabilityVariant::Notification,
            Tag::Devmem => CapabilityVariant::Devmem,
            Tag::AsidControl => CapabilityVariant::AsidControl,
            Tag::Endpoint => CapabilityVariant::Endpoint,
        };

        (Schedule::Keep, Ok(variant))
    }
}
