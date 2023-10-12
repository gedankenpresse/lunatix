use core::{mem::ManuallyDrop, ptr, sync::atomic::AtomicUsize};

use derivation_tree::caps::CapabilityIface;
use riscv::pt::PageTable;

use crate::caps::{Tag, Variant};

use super::{Capability, Error, VSpace};

#[derive(Copy, Clone)]
pub struct Asid {
    allocated: bool,
    pub id: usize,
    pub pt: *mut PageTable,
}

pub static ASID_MARKER: AtomicUsize = AtomicUsize::new(1);
pub static ASID_NONE: usize = 0;

pub struct AsidPool {
    asids: [Asid; 64],
}

unsafe impl Send for AsidPool {}
unsafe impl Sync for AsidPool {}

pub static mut ASID_POOL: AsidPool = AsidPool {
    asids: [Asid {
        allocated: false,
        id: 0,
        pt: ptr::null_mut(),
    }; 64],
};

pub struct AsidControl;

impl AsidControl {
    pub fn alloc_asid(&self) -> Result<&mut Asid, Error> {
        let pool = unsafe { &mut ASID_POOL };
        let asid = pool
            .asids
            .iter_mut()
            .find(|i| !i.allocated)
            .ok_or(Error::NoAsid)?;
        asid.allocated = true;
        asid.id = ASID_MARKER.fetch_add(1, core::sync::atomic::Ordering::SeqCst);
        Ok(asid)
    }
}

pub fn init_asid_control(slot: &mut Capability) {
    assert_eq!(slot.tag, Tag::Uninit);
    slot.tag = Tag::AsidControl;
    slot.variant = Variant {
        asid_control: ManuallyDrop::new(AsidControl),
    }
}

pub fn asid_control_assign(asid_control: &AsidControl, vspace: &mut VSpace) -> Result<(), Error> {
    let asid = asid_control.alloc_asid()?;
    asid.pt = vspace.root;
    vspace.asid = asid.id;
    Ok(())
}

pub struct AsidControlIface;

impl CapabilityIface<Capability> for AsidControlIface {
    type InitArgs = ();

    fn init(
        &self,
        target: &mut impl derivation_tree::AsStaticMut<Capability>,
        args: Self::InitArgs,
    ) {
        todo!()
    }

    fn copy(
        &self,
        src: &impl derivation_tree::AsStaticRef<Capability>,
        dst: &mut impl derivation_tree::AsStaticMut<Capability>,
    ) {
        todo!()
    }

    fn destroy(&self, target: &mut Capability) {
        todo!()
    }
}
