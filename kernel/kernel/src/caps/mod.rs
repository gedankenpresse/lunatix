pub mod asid;
pub mod cspace;
pub mod devmem;
pub mod irq;
pub mod irq_control;
pub mod memory;
pub mod notification;
pub mod page;
pub mod prelude;
pub mod task;
pub mod vspace;

use core::{marker::PhantomData, mem, mem::ManuallyDrop};

use derivation_tree::{
    tree::{TreeNodeData, TreeNodeOps},
    AsStaticMut, AsStaticRef, Correspondence,
};

pub use asid::{AsidControl, AsidControlIface};
pub use cspace::{CSpace, CSpaceIface};
pub use devmem::{Devmem, DevmemEntry, DevmemIface};
pub use irq::{Irq, IrqIface};
pub use irq_control::{IrqControl, IrqControlIface};
pub use memory::{Memory, MemoryIface};
pub use notification::{Notification, NotificationIface};
pub use page::{Page, PageIface};
pub use task::{Task, TaskIface};
pub use vspace::{VSpace, VSpaceIface};

pub use prelude::*;
pub use syscall_abi::Error;

#[derive(Copy, Clone)]
pub struct Uninit {}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[repr(u8)]
pub enum Tag {
    Uninit,
    Memory,
    CSpace,
    VSpace,
    Task,
    Page,
    IrqControl,
    Irq,
    Notification,
    Devmem,
    AsidControl,
}

pub union Variant {
    uninit: Uninit,
    memory: ManuallyDrop<Memory>,
    cspace: ManuallyDrop<CSpace>,
    vspace: ManuallyDrop<VSpace>,
    task: ManuallyDrop<Task>,
    page: ManuallyDrop<Page>,
    irq_control: ManuallyDrop<IrqControl>,
    irq: ManuallyDrop<Irq>,
    notification: ManuallyDrop<Notification>,
    devmem: ManuallyDrop<Devmem>,
    asid_control: ManuallyDrop<AsidControl>,
}

pub struct Capability {
    tag: Tag,
    tree_data: TreeNodeData<Self>,
    variant: Variant,
}

impl Capability {
    pub fn get_tag(&self) -> &Tag {
        &self.tag
    }
}

impl Correspondence for Capability {
    fn corresponds_to(&self, other: &Self) -> bool {
        match (self.tag, other.tag) {
            (Tag::Uninit, Tag::Uninit) => false,
            (Tag::Memory, Tag::Memory) => unsafe {
                self.variant.memory.corresponds_to(&other.variant.memory)
            },
            (Tag::CSpace, Tag::CSpace) => unsafe {
                self.variant.cspace.corresponds_to(&other.variant.cspace)
            },
            (Tag::VSpace, Tag::VSpace) => unsafe {
                self.variant.vspace.corresponds_to(&other.variant.vspace)
            },
            (Tag::Task, Tag::Task) => unsafe {
                self.variant.task.corresponds_to(&other.variant.task)
            },
            (Tag::Page, Tag::Page) => unsafe {
                self.variant.page.corresponds_to(&other.variant.page)
            },
            _ => false,
        }
    }
}

impl TreeNodeOps for Capability {
    fn get_tree_data(&self) -> &TreeNodeData<Self> {
        &self.tree_data
    }
}

macro_rules! cap_get_ref_mut {
    ($variant:ty, $tag:ident, $name:ident, $name_mut: ident) => {
        impl Capability {
            pub fn $name_mut<'a>(&'a mut self) -> Result<CapRefMut<'a, $variant>, ()> {
                if self.tag == Tag::$tag {
                    Ok(CapRefMut {
                        cap: self,
                        _type: PhantomData,
                    })
                } else {
                    Err(())
                }
            }

            pub fn $name<'a>(&'a self) -> Result<CapRef<'a, $variant>, ()> {
                if self.tag == Tag::$tag {
                    Ok(CapRef {
                        cap: self,
                        _type: PhantomData,
                    })
                } else {
                    Err(())
                }
            }
        }
    };
}

macro_rules! cap_get_inner_mut {
    ($typ:ty, $tag:ident, $variant:ident, $name:ident, $name_mut:ident) => {
        impl Capability {
            pub fn $name<'a>(&'a self) -> Result<&'a $typ, ()> {
                if self.tag == Tag::$tag {
                    Ok(unsafe { &self.variant.$variant })
                } else {
                    Err(())
                }
            }

            pub fn $name_mut<'a>(&'a mut self) -> Result<&'a mut $typ, ()> {
                if self.tag == Tag::$tag {
                    Ok(unsafe { &mut self.variant.$variant })
                } else {
                    Err(())
                }
            }
        }
    };
}

cap_get_ref_mut!(Task, Task, get_task, get_task_mut);
cap_get_inner_mut!(Task, Task, task, get_inner_task, get_inner_task_mut);
cap_get_ref_mut!(VSpace, VSpace, get_vspace, get_vspace_mut);
cap_get_inner_mut!(
    VSpace,
    VSpace,
    vspace,
    get_inner_vspace,
    get_inner_vspace_mut
);
cap_get_ref_mut!(CSpace, CSpace, get_cspace, get_cspace_mut);
cap_get_inner_mut!(
    CSpace,
    CSpace,
    cspace,
    get_inner_cspace,
    get_inner_cspace_mut
);

cap_get_ref_mut!(Memory, Memory, get_memory, get_memory_mut);
cap_get_inner_mut!(
    Memory,
    Memory,
    memory,
    get_inner_memory,
    get_inner_memory_mut
);

cap_get_ref_mut!(Page, Page, get_page, get_page_mut);
cap_get_inner_mut!(Page, Page, page, get_inner_page, get_inner_page_mut);

cap_get_ref_mut!(IrqControl, IrqControl, get_irq_control, get_irq_control_mut);
cap_get_inner_mut!(
    IrqControl,
    IrqControl,
    irq_control,
    get_inner_irq_control,
    get_inner_irq_control_mut
);

cap_get_ref_mut!(Irq, Irq, get_irq, get_irq_mut);
cap_get_inner_mut!(Irq, Irq, irq, get_inner_irq, get_inner_irq_mut);

cap_get_ref_mut!(
    Notification,
    Notification,
    get_notification,
    get_notification_mut
);
cap_get_inner_mut!(
    Notification,
    Notification,
    notification,
    get_inner_notification,
    get_inner_notification_mut
);
cap_get_ref_mut!(Devmem, Devmem, get_devmem, get_devmem_mut);

cap_get_inner_mut!(
    Devmem,
    Devmem,
    devmem,
    get_inner_devmem,
    get_inner_devmem_mut
);

cap_get_ref_mut!(
    AsidControl,
    AsidControl,
    get_asid_control,
    get_asid_control_mut
);

cap_get_inner_mut!(
    AsidControl,
    AsidControl,
    asid_control,
    get_inner_asid_control,
    get_inner_asid_control_mut
);

pub struct CapRef<'a, T> {
    pub cap: &'a Capability,
    _type: PhantomData<T>,
}

pub struct CapRefMut<'a, T> {
    cap: &'a mut Capability,
    _type: PhantomData<T>,
}

macro_rules! cap_ref_as_ref_impl {
    ($variant:ty, $name:ident) => {
        impl<'a> AsRef<$variant> for CapRef<'a, $variant> {
            fn as_ref(&self) -> &$variant {
                unsafe { &self.cap.variant.$name }
            }
        }

        impl<'a> AsRef<$variant> for CapRefMut<'a, $variant> {
            fn as_ref(&self) -> &$variant {
                unsafe { &self.cap.variant.$name }
            }
        }

        impl<'a> AsMut<$variant> for CapRefMut<'a, $variant> {
            fn as_mut(&mut self) -> &mut $variant {
                unsafe { &mut self.cap.variant.$name }
            }
        }
    };
}

cap_ref_as_ref_impl!(CSpace, cspace);
cap_ref_as_ref_impl!(VSpace, vspace);
cap_ref_as_ref_impl!(Memory, memory);
cap_ref_as_ref_impl!(Task, task);
cap_ref_as_ref_impl!(Page, page);

impl Default for Capability {
    fn default() -> Self {
        Self::empty()
    }
}

// TODO: remove
// TODO: This should be done via a cursor (where it is already implemented)
unsafe impl AsStaticRef<Capability> for Capability {
    fn as_static_ref(&self) -> &'static Capability {
        // Safety: This is safe because capabilities can only be retrieved by getting them from the derivation tree
        // which tracks lifetimes at runtime
        unsafe { mem::transmute(self) }
    }
}

unsafe impl AsStaticMut<Capability> for Capability {
    fn as_static_mut(&mut self) -> &'static mut Capability {
        // Safety: This is safe because capabilities can only be retrieved by getting them from the derivation tree
        // which tracks lifetimes at runtime
        unsafe { mem::transmute(self) }
    }
}

impl Capability {
    /// Create a new empty (in other words uninitialized) capability
    pub const fn empty() -> Self {
        Self {
            tag: Tag::Uninit,
            tree_data: unsafe { TreeNodeData::new() },
            variant: Variant { uninit: Uninit {} },
        }
    }
}
