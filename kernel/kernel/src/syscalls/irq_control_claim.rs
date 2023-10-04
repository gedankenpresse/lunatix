use crate::caps::{Capability, IrqControlIface, IrqIface, NotificationIface, Tag};
use crate::syscalls::utils;
use derivation_tree::caps::CapabilityIface;
use derivation_tree::tree::{CursorRefMut, TreeNodeOps};
use syscall_abi::irq_control_claim::IrqControlClaim;
use syscall_abi::{NoValue, SysError, SyscallBinding};

pub(super) fn sys_irq_control_claim(
    task: &mut CursorRefMut<'_, '_, Capability>,
    args: <IrqControlClaim as SyscallBinding>::CallArgs,
) -> <IrqControlClaim as SyscallBinding>::Return {
    // get basic caps from task
    let task = task.get_inner_task().unwrap();
    let mut cspace = task.get_cspace();
    let cspace = cspace.get_shared().unwrap();
    let cspace = cspace.get_inner_cspace().unwrap();

    // get valid irq-control cap from task
    let irq_control_cap =
        unsafe { utils::lookup_cap_mut(cspace, args.irq_control_addr, Tag::IrqControl) }.unwrap();

    // get valid notification cap from task
    let notification_cap =
        unsafe { utils::lookup_cap(cspace, args.notification_addr, Tag::Notification) }.unwrap();

    // get valid uninitialized target cap from task
    let irq_cap = unsafe { utils::lookup_cap_mut(cspace, args.irq_addr, Tag::Uninit) }.unwrap();

    // try to claim the given interrupt line
    match IrqControlIface.try_get_unclaimed(irq_control_cap, args.interrupt_line) {
        Err(_) => Err(SysError::UnknownError),
        Ok(irq_control_slot) => {
            // create a new irq capability in the slot intended for it
            IrqIface.init(irq_cap, args.interrupt_line);
            unsafe {
                irq_control_cap.insert_derivation(irq_cap);
            }

            // write a copy of the notification into the irq-control slot to claim it
            let irq_control_slot = unsafe { &mut *irq_control_slot };
            NotificationIface.copy(notification_cap, irq_control_slot);

            Ok(NoValue)
        }
    }
}
