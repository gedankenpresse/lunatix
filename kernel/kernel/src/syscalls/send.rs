use derivation_tree::AsStaticMut;
use syscall_abi::send::{Send, SendArgs};
use syscall_abi::{IntoRawSysRepsonse, NoValue, RawSyscallArgs};

use crate::sched::Schedule;
use crate::syscalls::SyscallContext;
use crate::{
    caps::{self, SyscallError},
    KernelContext,
};

use super::handler_trait::RawSyscallHandler;
use super::ipc;

pub(super) struct SendHandler;

impl RawSyscallHandler for SendHandler {
    type Syscall = Send;

    fn handle_raw(
        &mut self,
        kernel_ctx: &mut KernelContext,
        syscall_ctx: &mut SyscallContext<'_, '_>,
        raw_args: RawSyscallArgs,
    ) -> Schedule {
        // <<Self as SyscallHandler>::Syscall as SyscallBinding>::Return,
        let args = SendArgs::from(raw_args);
        let task_ptr = syscall_ctx.task.as_static_mut() as *mut _;
        let task = syscall_ctx.task.get_inner_task().unwrap();

        // increase the tasks program counter
        task.state.borrow_mut().frame.start_pc = syscall_ctx.trap_info.epc + 4;
        let mut cspace = task.get_cspace();
        let cspace = cspace.get_shared().unwrap();
        let cspace = cspace.get_inner_cspace().unwrap();

        let cap = unsafe {
            cspace
                .resolve_caddr(args.target)
                .unwrap() // TODO Handle error by returning InvalidCAddr
                .as_mut()
                .unwrap()
        };
        log::debug!("dispatching send to {:?} capability", cap.get_tag());
        let result = match cap.get_tag() {
            caps::Tag::Uninit => todo!("send for uninit unimplemented"),
            caps::Tag::Memory => ipc::mem::mem_send(cspace, cap, &args),
            caps::Tag::CSpace => todo!("send for cspace unimplemented"),
            caps::Tag::VSpace => todo!("send for vspace unimplemented"),
            caps::Tag::Task => ipc::task::task_send(cspace, cap.get_inner_task().unwrap(), &args),
            caps::Tag::Page => {
                ipc::page::page_send(cspace, cap.get_inner_page_mut().unwrap(), &args)
            }
            caps::Tag::IrqControl => ipc::irq::irq_control_send(kernel_ctx, cspace, cap, &args),
            caps::Tag::Irq => {
                ipc::irq::irq_send(kernel_ctx, cspace, cap.get_inner_irq().unwrap(), &args)
            }
            caps::Tag::Notification => todo!("send for notification unimplemented"),
            caps::Tag::Devmem => {
                ipc::devmem::devmem_send(cspace, cap.get_inner_devmem().unwrap(), &args)
            }
            caps::Tag::AsidControl => ipc::asid_control::asid_control_send(
                cspace,
                cap.get_inner_asid_control().unwrap(),
                &args,
            ),
            caps::Tag::Endpoint => {
                log::debug!("handling endpoint send");
                let (res, schedule) = ipc::endpoint::endpoint_send(
                    task_ptr,
                    task,
                    cap,
                    cap.get_inner_endpoint().unwrap(),
                );
                if let Some(res) = res {
                    task.state
                        .borrow_mut()
                        .frame
                        .write_syscall_return(res.into_response());
                }
                return schedule;
            }
        };

        match result {
            Ok(_) => {
                task.state
                    .borrow_mut()
                    .frame
                    .write_syscall_return(Ok(NoValue).into_response());
                Schedule::Keep
            }
            Err(e) => {
                task.state
                    .borrow_mut()
                    .frame
                    .write_syscall_return(Err::<NoValue, SyscallError>(e).into_response());
                Schedule::Keep
            }
        }
    }
}
