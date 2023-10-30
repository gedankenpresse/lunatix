use derivation_tree::tree::CursorRefMut;

use crate::{
    caps::{self, Capability, Error},
    SyscallContext,
};

pub fn sys_copy(
    _ctx: &mut SyscallContext,
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

    unsafe { caps::copy(src, target) };
    Ok(())
}
