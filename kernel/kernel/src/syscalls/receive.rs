use derivation_tree::AsStaticMut;
use syscall_abi::{
    receive::{Receive, ReceiveArgs, ReceiveReturn},
    IntoRawSysRepsonse, NoValue, SyscallError, SyscallResult,
};

use crate::{caps, sched::Schedule, syscalls::ipc, KernelContext};

use super::handler_trait::RawSyscallHandler;

pub(super) struct ReceiveHandler;

impl RawSyscallHandler for ReceiveHandler {
    type Syscall = Receive;

    fn handle_raw(
        &mut self,
        _kernel_ctx: &mut KernelContext,
        syscall_ctx: &mut super::SyscallContext<'_, '_>,
    ) -> Schedule {
        // <<Self as SyscallHandler>::Syscall as SyscallBinding>::Return,
        let raw_args = syscall_ctx.get_raw_args();
        let args = ReceiveArgs::from(raw_args);
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
        let result: SyscallResult<ReceiveReturn> = match cap.get_tag() {
            caps::Tag::Uninit => todo!(),
            caps::Tag::Memory => todo!(),
            caps::Tag::CSpace => todo!(),
            caps::Tag::VSpace => todo!(),
            caps::Tag::Task => todo!(),
            caps::Tag::Page => todo!(),
            caps::Tag::IrqControl => todo!(),
            caps::Tag::Irq => todo!(),
            caps::Tag::Notification => todo!(),
            caps::Tag::Devmem => todo!(),
            caps::Tag::AsidControl => todo!(),
            caps::Tag::Endpoint => {
                log::debug!("handling endpoint receive");
                let (res, schedule) = ipc::endpoint::endpoint_recv(
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
        #[allow(unreachable_code)]
        match result {
            Ok(r) => {
                task.state
                    .borrow_mut()
                    .frame
                    .write_syscall_return(Ok(r).into_response());
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
