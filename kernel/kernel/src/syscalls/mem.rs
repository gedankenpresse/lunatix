use crate::caps::{
    CSpace, CSpaceIface, Capability, Error, NotificationIface, PageIface, Tag, TaskIface,
    VSpaceIface,
};
use syscall_abi::identify::CapabilityVariant;

use super::utils;

pub fn mem_send(cspace: &CSpace, mem: &Capability, args: &[usize]) -> Result<(), Error> {
    log::info!("mem derive: {:?}", args);
    const DERIVE: usize = 1;
    match args[0] {
        DERIVE => mem_derive(
            cspace,
            mem,
            args[1],
            CapabilityVariant::try_from(args[2]).map_err(|_| Error::InvalidArg)?,
            args[3],
        ),
        _ => Err(Error::Unsupported),
    }
}

fn mem_derive(
    cspace: &CSpace,
    mem: &Capability,
    target: usize,
    variant: CapabilityVariant,
    size: usize,
) -> Result<(), Error> {
    let target_cap = unsafe { utils::lookup_cap_mut(cspace, target, Tag::Uninit)? };
    // derive the correct capability
    match variant {
        CapabilityVariant::Uninit => return Err(Error::InvalidArg),
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
    }
    Ok(())
}
