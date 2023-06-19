pub mod cspace;
pub mod memory;
pub mod task;

pub use cspace::{ CSpace };
use self::errors::OccupiedSlot;
pub use self::memory::{ Memory };
pub use task::{ Task };
pub use errors::Error;

pub enum Capability {
    CSpace(Cap<CSpace>),
    Memory(Cap<Memory>),
    Task(Cap<Task>),
    Uninit,
}

impl Default for Capability {
    fn default() -> Self {
        Self::Uninit
    }
}

impl From<Cap<CSpace>> for Capability {
    fn from(value: Cap<CSpace>) -> Self {
        Self::CSpace(value)
    }
}

impl From<Cap<Memory>> for Capability {
    fn from(value: Cap<Memory>) -> Self {
        Self::Memory(value)
    }
}

impl From<Cap<Task>> for Capability {
    fn from(value: Cap<Task>) -> Self {
        Self::Task(value)
    }
}

#[derive(Default)]
pub struct CSlot {
    cap: Capability
}

impl CSlot {
    pub (crate) fn set(&mut self, v: impl Into<Capability>) -> Result<(), OccupiedSlot> {
        match self.cap {
            Capability::Uninit => {
                self.cap = v.into();
                Ok(())
            },
            _ => Err(OccupiedSlot)
        }
    }
}

pub struct Cap<Type> {
    pub (crate) header: usize,
    // link field should be here
    pub (crate) content: Type,
}

impl<Type> core::ops::Deref for Cap<Type> {
    type Target = Type;

    fn deref(&self) -> &Self::Target {
        &self.content
    }
}

impl<Type> core::ops::DerefMut for Cap<Type> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.content
    }
}


impl<Type> Cap<Type> {
    pub fn from_content(content: Type) -> Self {
        Self {
            header: 0,
            content
        }
    }
}


mod errors {

    #[derive(Debug)]
    pub enum Error {
        InvalidCAddr,
        NoMem,
        OccupiedSlot,
    }

    impl From<InvalidCAddr> for Error {
        fn from(value: InvalidCAddr) -> Self {
            Self::InvalidCAddr
        }
    }

    impl From<NoMem> for Error {
        fn from(value: NoMem) -> Self {
            Self::NoMem
        }
    }

    impl From<OccupiedSlot> for Error {
        fn from(value: OccupiedSlot) -> Self {
            Self::OccupiedSlot
        }
    }

    pub struct InvalidCAddr;
    pub struct NoMem;

    pub struct OccupiedSlot;
}

