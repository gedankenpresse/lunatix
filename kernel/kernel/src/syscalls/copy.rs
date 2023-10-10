use derivation_tree::{caps::CapabilityIface, tree::CursorRefMut};

use crate::{
    caps::{
        AsidControlIface, CSpaceIface, Capability, DevmemIface, Error, IrqControlIface, IrqIface,
        MemoryIface, NotificationIface, PageIface, TaskIface, VSpaceIface,
    },
    SyscallContext,
};

pub fn sys_copy(
    ctx: &mut SyscallContext,
    task: &mut CursorRefMut<'_, '_, Capability>,
    args: &[usize; 7],
) -> Result<(), Error> {
    log::debug!("copy args: {:?}", args);
    let task = task.get_inner_task().unwrap();
    let mut cspace = task.get_cspace();
    let cspace = cspace.get_shared().unwrap();
    let cspace = cspace.get_inner_cspace().unwrap();

    let src = unsafe {
        cspace
            .lookup_raw(args[0])
            .ok_or(Error::InvalidCAddr)?
            .as_ref()
            .unwrap()
    };
    let target = unsafe {
        cspace
            .lookup_raw(args[1])
            .ok_or(Error::InvalidCAddr)?
            .as_mut()
            .unwrap()
    };

    match src.get_tag() {
        crate::caps::Tag::Uninit => {}
        crate::caps::Tag::Memory => MemoryIface.copy(src, target),
        crate::caps::Tag::CSpace => CSpaceIface.copy(src, target),
        crate::caps::Tag::VSpace => VSpaceIface.copy(src, target),
        crate::caps::Tag::Task => TaskIface.copy(src, target),
        crate::caps::Tag::Page => PageIface.copy(src, target),
        crate::caps::Tag::IrqControl => IrqControlIface.copy(src, target),
        crate::caps::Tag::Irq => IrqIface.copy(src, target),
        crate::caps::Tag::Notification => NotificationIface.copy(src, target),
        crate::caps::Tag::Devmem => DevmemIface.copy(src, target),
        crate::caps::Tag::AsidControl => AsidControlIface.copy(src, target),
    };
    Ok(())
}
