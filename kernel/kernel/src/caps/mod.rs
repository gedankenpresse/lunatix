pub mod cspace;
pub mod memory;
pub mod task;
pub mod vspace;

use core::cell::RefCell;

use self::errors::OccupiedSlot;
pub use self::memory::Memory;
pub use cspace::CSpace;
pub use errors::Error;
pub use task::Task;
pub use vspace::VSpace;

pub struct Chain<T> {
    pub prev: Link<T>,
    pub next: Link<T>,
    pub depth: usize,
}

pub struct Node<T> {
    pub chain: Chain<T>,
    pub elem: T,
}

pub type CNode = Node<Capability>;

pub type Link<T> = *mut Node<T>;
pub const fn no_link<T>() -> Link<T> {
    core::ptr::null_mut()
}

pub const fn no_chain<T>() -> Chain<T> {
    Chain {
        prev: no_link(),
        next: no_link(),
        depth: 0,
    }
}

pub enum Capability {
    CSpace(CSpace),
    Memory(Memory),
    Task(Task),
    VSpace(VSpace),
    Uninit,
}

#[repr(usize)]
pub enum Variant {
    Uninit = 0,
    Memory = 1,
    CSpace = 2,
    VSpace = 3,
    Task = 4,
}

impl Capability {
    pub(crate) fn get_variant(&self) -> Variant {
        match self {
            Capability::CSpace(_) => Variant::CSpace,
            Capability::Memory(_) => Variant::Memory,
            Capability::Task(_) => Variant::Task,
            Capability::VSpace(_) => Variant::VSpace,
            Capability::Uninit => Variant::Uninit,
        }
    }
}



impl<T> Node<T> {
    unsafe fn as_link(&self) -> Link<T> {
        return self as *const Node<T> as Link<T>;
    }

    fn parent(&mut self) -> Link<T> {
        unsafe {
            let mut cur = self.as_link();
            while !(*cur).chain.prev.is_null() && (*cur).chain.depth >= self.chain.depth {
                cur = (*cur).chain.prev;
            } 
            if (*cur).chain.depth >= self.chain.depth {
                return core::ptr::null_mut();
            };
            return cur;
        }
    }

    /// Inserts a capability as a sibling of a reference capability.
	/// Before:
	///
	///   prev         self        next
	/// +-------+   +-------+   +-------+
	/// |      ---> |      *--> |       |
	/// |  d=x  | <--- d=q  | <--* d=y  |
	/// +-------+   +-------+   +-------+
	///
	/// After:
	///
	///   prev         self        copy        next
	/// +-------+   +-------+   +-------+   +-------+
	/// |      ---> |      *--> |      *--> |       |
	/// |  d=x  | <--- d=q  | <--* d=q  | <--* d=y  |
	/// +-------+   +-------+   +-------+   +-------+
    fn link_copy(&mut self, copy: Link<T>) {
        unsafe {
            if !self.chain.next.is_null() {
                (*self.chain.next).chain.prev = copy;
            }

            let copychain = &mut copy.as_mut().unwrap().chain;
            assert!(copychain.next.is_null());
            assert!(copychain.prev.is_null());
            copychain.next = self.chain.next;
            copychain.prev = self.as_link();
            copychain.depth = self.chain.depth;
            self.chain.next = copy;
        }
    }

    /// Inserts a capability as a child of another capability.
	/// Process is identical to a copy, but the depth is increased
	///
	/// Before:
	///
	///   prev       parent       next
	/// +-------+   +-------+   +-------+
	/// |      ---> |      *--> |       |
	/// |  d=x  | <--- d=q  | <--* d=y  |
	/// +-------+   +-------+   +-------+
	///
	/// After:
	///
	///   prev       parent       child       next
	/// +-------+   +-------+   +-------+   +-------+
	/// |      ---> |      *--> |      *--> |       |
	/// |  d=x  | <--- d=q  | <--* d=q+1| <--* d=y  |
	/// +-------+   +-------+   +-------+   +-------+

    fn link_derive(&mut self, child: Link<T>) {
        unsafe { 
            let parent = self.get_last_copy();
            parent.as_mut().unwrap().link_copy(child);
            (*child).chain.depth += 1;
        }
    }

    /// Removes a capability from the derivation list.
    fn unlink(&mut self) {
        todo!();
    }

    fn get_last_copy(&mut self) -> Link<T> {
        // TODO: fix
        unsafe { self.as_link() }
    }

    fn get_first_copy(&mut self) -> Link<T> {
        todo!()
    }

    /// Returns true if this capability has any children.
    fn has_children(&self) -> bool {
        unsafe { 
            let root = (*self.as_link()).get_last_copy().as_ref().unwrap();
            if let Some(next) = root.chain.next.as_ref() {
               if next.chain.depth > root.chain.depth {
                return true;
               }
            }
            return false;
        }
    }
    /// Returns true if this capability is the last reference to the underlying object.
    fn is_final(&self) -> bool {
        todo!();
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
    }
}

cap_from_node_impl!(CSpace, CSpace);
cap_from_node_impl!(Memory, Memory);
cap_from_node_impl!(Task, Task);
cap_from_node_impl!(VSpace, VSpace);

pub struct NodeRefMut<'n, T> {
    pub chain: &'n mut Chain<Capability>,
    pub elem: &'n mut T
}

macro_rules! cap_get_mut {
    ($v:ident, $n: ident, $t:ty) => {
        impl Node<Capability> {
            pub fn $n(&mut self) -> Result<NodeRefMut<$t>, errors::InvalidCap> {
                let chainref = &mut self.chain;
                let elemref = &mut self.elem;
                match elemref {
                    Capability::$v(m) => Ok(NodeRefMut { chain: chainref, elem: m }),
                    _ => Err(errors::InvalidCap),
                }
            }
        }
    }
}

cap_get_mut!(Memory, get_memory_mut, Memory);
cap_get_mut!(Task, get_task_mut, Task);
cap_get_mut!(VSpace, get_vspace_mut, VSpace);
cap_get_mut!(CSpace, get_cspace_mut, CSpace);

pub struct CSlot {
    pub cap: Node<Capability>,
}

impl CNode {
    pub fn send(&mut self, label: usize, caps: &[Option<&RefCell<CSlot>>], params: &[usize]) -> Result<usize, Error> {
        match &mut self.elem {
            Capability::CSpace(_cspace) => todo!("implement cspace send"),
            Capability::Memory(_mem) => Memory::send(self, label, caps, params),
            Capability::Task(_task) => todo!("implement task send"),
            Capability::VSpace(_vspace) => todo!("implement vspace send"),
            Capability::Uninit => Err(Error::InvalidCap),
        }
    }
}



impl CSlot {
    pub(crate) fn set(&mut self, v: impl Into<Capability>) -> Result<(), OccupiedSlot> {
        match self.cap.elem {
            Capability::Uninit => {
                self.cap.elem = v.into();
                Ok(())
            }
            _ => Err(OccupiedSlot),
        }
    }

    pub const fn empty() -> Self {
        Self {
            cap: Node {
                chain: no_chain(),
                elem: Capability::Uninit,
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
        InvalidOp = 5,
        InvalidArg = 6,
        AliasingCSlot = 7,
        InvalidReturn = 8,
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
        }
    }

    macro_rules! singleton_variant {
        ($t:ident) => {
            #[derive(Debug)]
            pub struct $t;
        }
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
