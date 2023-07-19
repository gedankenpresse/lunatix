use core::ops::Deref;

#[repr(transparent)]
pub struct SharedRef<T> {
    pub(crate) value: *const T,
}

impl<T> Copy for SharedRef<T> {}
impl<T> Clone for SharedRef<T> {
    fn clone(&self) -> Self {
        Self { value: self.value }
    }
}

impl<T> Deref for SharedRef<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.value.as_ref().unwrap() }
    }
}
