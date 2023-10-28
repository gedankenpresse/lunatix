use derivation_tree::{caps::CapabilityIface, tree::TreeNodeOps};
use syscall_abi::send::SendArgs;

use crate::{
    arch_specific::plic::PLIC,
    caps::{self, CSpace, Capability, Irq, IrqControlIface, IrqIface, NotificationIface, Tag},
    SyscallContext,
};

use super::utils;

pub fn irq_send(
    ctx: &mut SyscallContext,
    cspace: &CSpace,
    irq: &Irq,
    args: &SendArgs,
) -> Result<(), caps::Error> {
    const COMPLETE: usize = 0;
    match args.label() {
        COMPLETE => sys_irq_complete(cspace, irq, ctx.plic),
        _ => Err(caps::Error::Unsupported),
    }
}

pub(super) fn sys_irq_complete(
    cspace: &CSpace,
    irq: &Irq,
    plic: &mut PLIC,
) -> Result<(), caps::Error> {
    let interrupt_line = irq.interrupt_line;

    // mark the interrupt as completed
    log::debug!("marking interrupt 0x{:x} as complete", interrupt_line);

    // TODO: figure out which context to complete
    plic.complete(1, interrupt_line as u32);

    Ok(())
}

pub fn irq_control_send(
    ctx: &mut SyscallContext,
    cspace: &CSpace,
    irq_control: &mut Capability,
    args: &SendArgs,
) -> Result<(), caps::Error> {
    const REGISTER_IRQ: usize = 0;
    match args.label() {
        REGISTER_IRQ => irq_control_claim(cspace, irq_control, &mut ctx.plic, args),
        _ => Err(caps::Error::Unsupported),
    }
}

// TODO: use typed irq_control cap here
fn irq_control_claim(
    cspace: &CSpace,
    irq_control: &mut Capability,
    plic: &mut PLIC,
    args: &SendArgs,
) -> Result<(), caps::Error> {
    let [notification_addr, irq_addr] = args.cap_args() else {
        panic!("not enough cap args")
    };

    let interrupt_line = args.data_args()[0];
    // get valid notification cap from task
    let notification_cap =
        unsafe { utils::lookup_cap(cspace, *notification_addr, Tag::Notification) }.unwrap();

    // get valid uninitialized target cap from task
    let irq_cap = unsafe { utils::lookup_cap_mut(cspace, *irq_addr, Tag::Uninit) }.unwrap();

    // try to claim the given interrupt line
    match IrqControlIface.try_get_unclaimed(irq_control, interrupt_line) {
        Err(e) => Err(e),
        Ok(irq_control_slot) => {
            // create a new irq capability in the slot intended for it
            IrqIface.init(irq_cap, interrupt_line);
            unsafe {
                irq_control.insert_derivation(irq_cap);
            }

            // write a copy of the notification into the irq-control slot to claim it
            let irq_control_slot = unsafe { &mut *irq_control_slot };
            NotificationIface.copy(notification_cap, irq_control_slot);

            // activate the specified interrupt line in the PLIC
            // we currently only run the first hart in supervisor mode, which corresponds to
            // qemu_virt:  context 1
            // qemu_sifive_u: context 2

            // TODO: determine which context(s) whe should enable
            plic.enable_interrupt(interrupt_line as u32, 0);
            plic.enable_interrupt(interrupt_line as u32, 1);
            plic.enable_interrupt(interrupt_line as u32, 2);
            plic.enable_interrupt(interrupt_line as u32, 3);

            // we use priority 2 because we set the interrupt threshold to 1 in plic initialization
            plic.set_priority(interrupt_line as u32, 2);

            Ok(())
        }
    }
}
