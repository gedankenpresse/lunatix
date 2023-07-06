pub mod cspace;
pub mod memory;
pub mod task;
pub mod vspace;

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


pub struct CSlot {
    pub cap: Node<Capability>,
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

    #[derive(Debug)]
    pub enum Error {
        InvalidCAddr,
        NoMem,
        OccupiedSlot,
        InvalidCap,
    }

    impl From<InvalidCAddr> for Error {
        fn from(_value: InvalidCAddr) -> Self {
            Self::InvalidCAddr
        }
    }

    impl From<NoMem> for Error {
        fn from(_value: NoMem) -> Self {
            Self::NoMem
        }
    }

    impl From<OccupiedSlot> for Error {
        fn from(_value: OccupiedSlot) -> Self {
            Self::OccupiedSlot
        }
    }

    impl From<InvalidCap> for Error {
        fn from(_value: InvalidCap) -> Self {
            Self::InvalidCap
        }
    }

    #[derive(Debug)]
    pub struct InvalidCAddr;

    #[derive(Debug)]
    pub struct NoMem;

    #[derive(Debug)]
    pub struct OccupiedSlot;

    #[derive(Debug)]
    pub struct InvalidCap;
}
