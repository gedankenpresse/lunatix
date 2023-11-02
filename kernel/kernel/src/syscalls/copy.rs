use derivation_tree::tree::CursorRefMut;

use crate::{
    caps::{self, Capability, SyscallError},
    SyscallContext,
};

pub fn sys_copy(
    _ctx: &mut SyscallContext,
    task: &mut CursorRefMut<'_, '_, Capability>,
    args: &[usize; 7],
) -> Result<(), SyscallError> {
    log::debug!("copy args: {:?}", args);
    let task = task.get_inner_task().unwrap();
    let mut cspace = task.get_cspace();
    let cspace = cspace.get_shared().unwrap();
    let cspace = cspace.get_inner_cspace().unwrap();

    let src = unsafe {
        cspace
            .resolve_caddr(args[0].into())
            .ok_or(SyscallError::InvalidCAddr)?
            .as_ref()
            .unwrap()
    };
    let target = unsafe {
        cspace
            .resolve_caddr(args[1].into())
            .ok_or(SyscallError::InvalidCAddr)?
            .as_mut()
            .unwrap()
    };

    unsafe { caps::copy(src, target) };
    Ok(())
}
