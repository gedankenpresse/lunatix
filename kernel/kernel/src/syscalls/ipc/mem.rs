use crate::caps::endpoint::EndpointIface;
use crate::caps::{
    CSpace, CSpaceIface, Capability, NotificationIface, PageIface, SyscallError, Tag, TaskIface,
    VSpaceIface,
};
use syscall_abi::identify::CapabilityVariant;
use syscall_abi::send::SendArgs;
use syscall_abi::CAddr;

use super::super::utils;

pub fn mem_send(cspace: &CSpace, mem: &Capability, args: &SendArgs) -> Result<(), SyscallError> {
    const DERIVE: usize = 1;
    match args.label() {
        DERIVE => mem_derive(
            cspace,
            mem,
            args.cap_args()[0],
            CapabilityVariant::try_from(args.data_args()[0])
                .map_err(|_| SyscallError::InvalidArg)?,
            args.data_args()[1],
        ),
        _ => Err(SyscallError::Unsupported),
    }
}

fn mem_derive(
    cspace: &CSpace,
    mem: &Capability,
    target: CAddr,
    variant: CapabilityVariant,
    size: usize,
) -> Result<(), SyscallError> {
    let target_cap = unsafe { utils::lookup_cap_mut(cspace, target, Tag::Uninit)? };

    // derive the correct capability
    match variant {
        CapabilityVariant::Uninit => return Err(SyscallError::InvalidArg),
        CapabilityVariant::Memory => unimplemented!("memory cannot yet be derived"),
        CapabilityVariant::CSpace => {
            CSpaceIface.derive(mem, target_cap, size);
        }
        CapabilityVariant::VSpace => {
            VSpaceIface.derive(mem, target_cap);
        }
        CapabilityVariant::Task => {
            TaskIface.derive(mem, target_cap);
        }
        CapabilityVariant::Page => {
            PageIface.derive(mem, target_cap);
        }
        CapabilityVariant::IrqControl => {
            todo!("signal that deriving irq-control from mem is not supported")
        }
        CapabilityVariant::Irq => todo!("signal that deriving irq from mem is not supported"),
        CapabilityVariant::Notification => {
            NotificationIface.derive(mem, target_cap);
        }
        CapabilityVariant::Devmem => todo!("cant derive devmem"),
        CapabilityVariant::AsidControl => todo!("cant derive asid_control"),
        CapabilityVariant::Endpoint => EndpointIface.derive(mem, target_cap),
    }
    Ok(())
}
