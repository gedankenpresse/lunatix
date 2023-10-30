use derivation_tree::tree::CursorRefMut;

use crate::{
    caps::{self, Capability, Error},
    SyscallContext,
};

pub fn sys_destroy(
    _ctx: &mut SyscallContext,
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
            .lookup_raw(args[0].into())
            .ok_or(Error::InvalidCAddr)?
            .as_mut()
            .unwrap()
    };

    unsafe { caps::destroy(target) };
    Ok(())
}
