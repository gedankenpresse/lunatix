use core::{mem::ManuallyDrop, ptr, sync::atomic::AtomicUsize};

use derivation_tree::{caps::CapabilityIface, tree::TreeNodeOps};
use riscv::pt::PageTable;

use crate::caps::{Tag, Uninit, Variant};

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

impl AsidPool {
    pub fn alloc_asid(&mut self) -> Result<&mut Asid, Error> {
        let asid = self
            .asids
            .iter_mut()
            .find(|i| !i.allocated)
            .ok_or(Error::NoAsid)?;
        asid.allocated = true;
        asid.id = ASID_MARKER.fetch_add(1, core::sync::atomic::Ordering::SeqCst);
        Ok(asid)
    }

    pub fn find_asid(&mut self, id: usize) -> Result<&Asid, Error> {
        self.asids
            .iter()
            .find(|i| i.allocated && i.id == id)
            .ok_or(Error::NoAsid)
    }
}

pub struct AsidControl;

impl AsidControl {
    pub fn alloc_asid(&self) -> Result<&mut Asid, Error> {
        let pool = unsafe { &mut ASID_POOL };
        pool.alloc_asid()
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
        _target: &mut impl derivation_tree::AsStaticMut<Capability>,
        _args: Self::InitArgs,
    ) {
        todo!()
    }

    fn copy(
        &self,
        src: &impl derivation_tree::AsStaticRef<Capability>,
        dst: &mut impl derivation_tree::AsStaticMut<Capability>,
    ) {
        let src = src.as_static_ref();
        let dst = dst.as_static_mut();
        assert_eq!(src.tag, Tag::AsidControl);
        assert_eq!(dst.tag, Tag::Uninit);

        dst.tag = Tag::AsidControl;
        dst.variant.asid_control = ManuallyDrop::new(AsidControl {});

        unsafe { src.insert_copy(dst) };
    }

    fn destroy(&self, target: &mut Capability) {
        assert_eq!(target.tag, Tag::AsidControl);

        // Note: AsidControl has no local state,
        // So we just drop the ZST here.
        // Global state should not be invalidated, because Page/Vspace can still refer to Asids.

        target.tree_data.unlink();
        target.tag = Tag::Uninit;
        target.variant.uninit = Uninit {};
    }
}
