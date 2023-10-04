use crate::caps::{Capability, Tag};
use crate::syscalls::{utils, SyscallContext};
use derivation_tree::tree::CursorRefMut;
use syscall_abi::irq_complete::IrqComplete;
use syscall_abi::{NoValue, SyscallBinding};

pub(super) fn sys_irq_complete(
    task: &mut CursorRefMut<'_, '_, Capability>,
    ctx: &mut SyscallContext,
    args: <IrqComplete as SyscallBinding>::CallArgs,
) -> <IrqComplete as SyscallBinding>::Return {
    // get basic caps from task
    let task = task.get_inner_task().unwrap();
    let mut cspace = task.get_cspace();
    let cspace = cspace.get_shared().unwrap();
    let cspace = cspace.get_inner_cspace().unwrap();

    // get valid irq cap from task
    let irq_cap = unsafe { utils::lookup_cap_mut(cspace, args.irq_addr, Tag::Irq) }.unwrap();
    let interrupt_line = irq_cap.get_inner_irq().unwrap().interrupt_line;

    // mark the interrupt as completed
    log::debug!("marking interrupt 0x{:x} as complete", interrupt_line);
    ctx.plic.complete(1, interrupt_line as u32);

    Ok(NoValue)
}
