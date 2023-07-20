pub mod cspace;
pub mod iface;
pub mod memory;
pub mod page;
pub mod task;
pub mod vspace;

use core::cell::{Ref, RefMut};

pub use self::memory::{Memory, MemoryIface};
use self::{errors::OccupiedSlot, iface::CapabilityInterface};
pub use cspace::{CSpace, CSpaceIface};
pub use page::{Page, PageIface};
pub use task::{Task, TaskIface};
pub use vspace::{VSpace, VspaceIface};

pub use errors::Error;
pub use iface::UninitIface;

pub type CNode = derivation_tree::Slot<Capability>;

pub enum Capability {
    CSpace(CSpace),
    Memory(Memory),
    Task(Task),
    VSpace(VSpace),
    Page(Page),
    Uninit,
}

#[repr(usize)]
#[derive(Copy, Clone)]
pub enum Variant {
    Uninit(UninitIface) = 0,
    Memory(MemoryIface) = 1,
    CSpace(CSpaceIface) = 2,
    VSpace(VspaceIface) = 3,
    Task(TaskIface) = 4,
    Page(PageIface) = 5,
}

impl Variant {
    #[inline(always)]
    pub fn discriminant(&self) -> usize {
        // SAFETY: Because `Self` is marked `repr(usize)`, its layout is a `repr(C)` `union`
        // between `repr(C)` structs, each of which has the `u8` discriminant as its first
        // field, so we can read the discriminant without offsetting the pointer.
        unsafe { *(self as *const Variant).cast::<usize>() }
    }
}

impl TryFrom<usize> for Variant {
    type Error = Error;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Uninit(UninitIface)),
            1 => Ok(Self::Memory(MemoryIface)),
            2 => Ok(Self::CSpace(CSpaceIface)),
            3 => Ok(Self::VSpace(VspaceIface)),
            4 => Ok(Self::Task(TaskIface)),
            _ => Err(Error::InvalidArg),
        }
    }
}

impl Capability {
    pub(crate) fn get_variant(&self) -> Variant {
        match self {
            Capability::CSpace(_) => Variant::CSpace(CSpaceIface),
            Capability::Memory(_) => Variant::Memory(MemoryIface),
            Capability::Task(_) => Variant::Task(TaskIface),
            Capability::VSpace(_) => Variant::VSpace(VspaceIface),
            Capability::Page(_) => Variant::Page(PageIface),
            Capability::Uninit => Variant::Uninit(UninitIface),
        }
    }
}

impl Default for Capability {
    fn default() -> Self {
        Self::Uninit
    }
}

macro_rules! cap_from_node_impl {
    ($v:ident, $t:ty) => {
        impl From<$t> for Capability {
            fn from(value: $t) -> Self {
                Self::$v(value)
            }
        }
    };
}

cap_from_node_impl!(CSpace, CSpace);
cap_from_node_impl!(Memory, Memory);
cap_from_node_impl!(Task, Task);
cap_from_node_impl!(VSpace, VSpace);
cap_from_node_impl!(Page, Page);

macro_rules! cap_get_mut {
    ($v:ident, $n: ident, $t:ty) => {
        impl CSlot {
            pub fn $n(&self) -> Result<RefMut<$t>, errors::InvalidCap> {
                let val = self.cap.get();
                match RefMut::filter_map(val.borrow_mut(), |cap| match cap {
                    Capability::$v(m) => Some(m),
                    _ => None,
                }) {
                    Ok(v) => Ok(v),
                    Err(_) => Err(errors::InvalidCap),
                }
            }
        }
    };
}

macro_rules! cap_get {
    ($v:ident, $n: ident, $t:ty) => {
        impl CSlot {
            pub fn $n(&self) -> Result<Ref<$t>, errors::InvalidCap> {
                let val = self.cap.get();
                match Ref::filter_map(val.borrow(), |cap| match cap {
                    Capability::$v(m) => Some(m),
                    _ => None,
                }) {
                    Ok(v) => Ok(v),
                    Err(_) => Err(errors::InvalidCap),
                }
            }
        }
    };
}

cap_get_mut!(Memory, get_memory_mut, Memory);
cap_get_mut!(Task, get_task_mut, Task);
cap_get_mut!(VSpace, get_vspace_mut, VSpace);
cap_get_mut!(CSpace, get_cspace_mut, CSpace);

cap_get!(CSpace, get_cspace, CSpace);

pub struct CSlot {
    // TODO: put refcell in slot or in derivation tree node? maybe both?
    cap: CNode,
}

impl CSlot {
    pub fn get_variant(&self) -> Variant {
        if self.cap.is_uninit() {
            return Variant::Uninit(UninitIface);
        }
        self.cap.get().borrow().get_variant()
    }

    pub fn is_uninit(&self) -> bool {
        return self.get_variant().discriminant() as usize
            == Variant::Uninit(UninitIface).discriminant();
    }

    pub fn send(
        &self,
        label: usize,
        caps: &[Option<&CSlot>],
        params: &[usize],
    ) -> Result<usize, Error> {
        let variant = self.cap.get().borrow().get_variant();
        match variant {
            Variant::CSpace(_) => todo!("implement cspace send"),
            Variant::Memory(_) => Memory::send(self, label, caps, params),
            Variant::Task(_) => todo!("implement task send"),
            Variant::VSpace(_) => todo!("implement vspace send"),
            Variant::Page(_) => todo!("implement page compare"),
            Variant::Uninit(_) => Err(Error::InvalidCap),
        }
    }

    /// sets the slot to given value.
    /// you propably want to panic on this error, because if the slot is occupied, you have to undo all the work to produce the value
    /// asserting unoccupied slot beforehand and using panic as a check seems better.
    pub(crate) fn set(&self, v: impl Into<Capability>) -> Result<(), OccupiedSlot> {
        self.cap.set(v.into()).ok().ok_or(OccupiedSlot)
    }

    pub const fn empty() -> Self {
        Self {
            cap: CNode::uninit(),
        }
    }

    pub fn derive(
        &self,
        target: &CSlot,
        f: impl FnOnce(&mut Memory) -> Result<Capability, Error>,
    ) -> Result<(), Error> {
        log::debug!("CSlot::derive derive_link");
        self.cap.derive_link(&target.cap);
        log::debug!("CSlot::derive get memory");
        let res = match self.get_memory_mut() {
            Ok(mut cap) => {
                let res = match f(&mut cap) {
                    Err(e) => Err(e),
                    Ok(cap) => target.set(cap).map_err(Into::into),
                };
                res
            }
            Err(e) => Err(e.into()),
        };
        match res {
            Ok(()) => Ok(()),
            Err(e) => {
                todo!("unlink target, error: {:?}", e);
                #[allow(unreachable_code)]
                Err(e)
            }
        }
    }
}

mod errors {
    #[repr(usize)]
    #[derive(Debug)]
    pub enum Error {
        InvalidCAddr = 1,
        NoMem = 2,
        OccupiedSlot = 3,
        InvalidCap = 4,
        InvalidArg = 6,
        AliasingCSlot = 7,
        InvalidReturn = 8,
        Unsupported = 9,
    }

    /// macro to implement From Instances from Singletons to Error
    /// invoking with `err_from_impl!(Variant, Type)` results in an impl
    /// that converts Type to Variant
    macro_rules! err_from_impl {
        ($v:ident, $t:ty) => {
            impl From<$t> for Error {
                fn from(_value: $t) -> Self {
                    Self::$v
                }
            }
        };
    }

    macro_rules! singleton_variant {
        ($t:ident) => {
            #[derive(Debug)]
            pub struct $t;
        };
    }

    singleton_variant!(InvalidCAddr);
    singleton_variant!(NoMem);
    singleton_variant!(OccupiedSlot);
    singleton_variant!(InvalidCap);

    err_from_impl!(InvalidCAddr, InvalidCAddr);
    err_from_impl!(NoMem, NoMem);
    err_from_impl!(OccupiedSlot, OccupiedSlot);
    err_from_impl!(InvalidCap, InvalidCap);

    err_from_impl!(AliasingCSlot, core::cell::BorrowMutError);
    err_from_impl!(AliasingCSlot, core::cell::BorrowError);
}
