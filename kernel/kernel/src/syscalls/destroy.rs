use derivation_tree::{caps::CapabilityIface, tree::CursorRefMut};

use crate::{
    caps::{
        AsidControl, AsidControlIface, CSpaceIface, Capability, DevmemIface, Error,
        IrqControlIface, IrqIface, MemoryIface, NotificationIface, PageIface, TaskIface,
        VSpaceIface,
    },
    SyscallContext,
};

pub fn sys_destroy(
    ctx: &mut SyscallContext,
    task: &mut CursorRefMut<'_, '_, Capability>,
    args: &[usize; 7],
) -> Result<(), Error> {
    log::debug!("send args: {:?}", args);
    let task = task.get_inner_task().unwrap();
    let mut cspace = task.get_cspace();
    let cspace = cspace.get_shared().unwrap();
    let cspace = cspace.get_inner_cspace().unwrap();

    let target = unsafe {
        cspace
            .lookup_raw(args[0])
            .ok_or(Error::InvalidCAddr)?
            .as_mut()
            .unwrap()
    };

    match target.get_tag() {
        crate::caps::Tag::Uninit => {}
        crate::caps::Tag::Memory => MemoryIface.destroy(target),
        crate::caps::Tag::CSpace => CSpaceIface.destroy(target),
        crate::caps::Tag::VSpace => VSpaceIface.destroy(target),
        crate::caps::Tag::Task => TaskIface.destroy(target),
        crate::caps::Tag::Page => PageIface.destroy(target),
        crate::caps::Tag::IrqControl => IrqControlIface.destroy(target),
        crate::caps::Tag::Irq => IrqIface.destroy(target),
        crate::caps::Tag::Notification => NotificationIface.destroy(target),
        crate::caps::Tag::Devmem => DevmemIface.destroy(target),
        crate::caps::Tag::AsidControl => AsidControlIface.destroy(target),
    };
    Ok(())
}
