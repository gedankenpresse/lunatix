use crate::{AsStaticMut, AsStaticRef};

#[repr(transparent)]
pub struct UninitSlot<'a, T> {
    slot: &'a mut T,
}

impl<'a, T> UninitSlot<'a, T> {
    pub unsafe fn new(slot: &'a mut T) -> Self {
        Self { slot }
    }
}

unsafe impl<T> AsStaticRef<T> for UninitSlot<'_, T> {
    fn as_static_ref(&self) -> &'static T {
        unsafe { &*(self.slot as *const _) }
    }
}

unsafe impl<T> AsStaticMut<T> for UninitSlot<'_, T> {
    fn as_static_mut(&mut self) -> &'static mut T {
        unsafe { &mut *(self.slot as *mut _) }
    }
}
