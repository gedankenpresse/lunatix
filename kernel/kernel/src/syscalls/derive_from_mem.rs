use crate::caps::{
    CSpaceIface, Capability, NotificationIface, PageIface, Tag, TaskIface, VSpaceIface,
};
use derivation_tree::tree::CursorRefMut;
use syscall_abi::derive_from_mem::DeriveFromMem;
use syscall_abi::identify::CapabilityVariant;
use syscall_abi::{NoValue, SysError, SyscallBinding};

use super::utils;

pub(super) fn sys_derive_from_mem(
    task: &mut CursorRefMut<'_, '_, Capability>,
    args: <DeriveFromMem as SyscallBinding>::CallArgs,
) -> <DeriveFromMem as SyscallBinding>::Return {
    // get basic caps from task
    let task = task.get_inner_task().unwrap();
    let mut cspace = task.get_cspace();
    let cspace = cspace.get_shared().unwrap();
    let cspace = cspace.get_inner_cspace().unwrap();

    // get valid memory cap from task
    let mem_cap = unsafe { utils::lookup_cap(cspace, args.src_mem, Tag::Memory) }?;

    // get valid uninitialized target cap from task
    let target_cap = unsafe { utils::lookup_cap_mut(cspace, args.target_slot, Tag::Uninit) }?;

    // derive the correct capability
    match args.target_cap {
        CapabilityVariant::Uninit => return Err(SysError::ValueInvalid),
        CapabilityVariant::Memory => unimplemented!("memory cannot yet be derived"),
        CapabilityVariant::CSpace => {
            CSpaceIface.derive(mem_cap, target_cap, args.size.unwrap());
        }
        CapabilityVariant::VSpace => {
            VSpaceIface.derive(mem_cap, target_cap);
        }
        CapabilityVariant::Task => {
            TaskIface.derive(mem_cap, target_cap);
        }
        CapabilityVariant::Page => {
            PageIface.derive(mem_cap, target_cap);
        }
        CapabilityVariant::IrqControl => {
            todo!("signal that deriving irq-control from mem is not supported")
        }
        CapabilityVariant::Irq => todo!("signal that deriving irq from mem is not supported"),
        CapabilityVariant::Notification => {
            NotificationIface.derive(mem_cap, target_cap);
        }
        CapabilityVariant::Devmem => todo!("cant derive devmem"),
    }
    Ok(NoValue)
}
