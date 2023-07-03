use crate::caps::memory::Memory;
use crate::caps::CapHolder;
use core::mem::MaybeUninit;
use core::ptr;

#[derive(Debug, Default, Copy, Clone)]
pub enum CSlot {
    #[default]
    Empty,
    Memory(*mut CapHolder<Memory>),
    CSpace(*mut CapHolder<CSpace>),
}

pub struct CSpace {
    slots: [CSlot; 32],
}

impl CSpace {
    /// Create a new empty CSpace
    pub unsafe fn new() -> Self {
        Self {
            slots: [CSlot::Empty; 32],
        }
    }

    /// Initialize a cspace located at the given memory location as an empty one.
    pub unsafe fn init(ptr: *mut MaybeUninit<CSpace>) {
        ptr.cast::<CSpace>().write(Self::new());
    }

    pub fn get(&self, i: usize) -> Option<&CSlot> {
        self.slots.get(i)
    }

    pub fn get_mut(&mut self, i: usize) -> Option<&mut CSlot> {
        self.slots.get_mut(i)
    }
}
