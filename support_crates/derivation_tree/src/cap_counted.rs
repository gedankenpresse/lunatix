use allocators::Box;
use core::mem::ManuallyDrop;
use core::ops::Deref;
use core::ptr;

/// A container for a value whose references are counted by the capabilities containing it.
///
/// This can be thought of as similar to an [`Rc`](std::rc::Rc) except that it does not count references to the
/// contained value internally but is managed externally through the capabilities using the reference.
///
/// When no capabilities need the contained value anymore, [`destroy()`](Self::destroy) must be called to
/// drop the contained value.
pub struct CapCounted<'alloc, 'mem, T: ?Sized>(ManuallyDrop<Box<'alloc, 'mem, T>>);

impl<'alloc, 'mem, T: ?Sized> CapCounted<'alloc, 'mem, T> {
    /// Turn a box into a CapCounted variable
    pub fn from_box(value: Box<'alloc, 'mem, T>) -> Self {
        Self(ManuallyDrop::new(value))
    }

    /// Manually drop the contained value.
    ///
    /// # Safety
    /// This function runs the destructor of the contained value. Other than changes made by the destructor itself,
    /// the memory is left unchanged, and so as far as the compiler is concerned still holds a bit-pattern which is
    /// valid for the type `T`.
    ///
    /// However, this “zombie” value should not be exposed to safe code, and this function should not be called more
    /// than once.
    /// To use a value after it’s been dropped, or drop a value multiple times, can cause Undefined Behavior
    /// (depending on what drop does).
    /// This is normally prevented by the type system, but users of CapCounted must uphold those guarantees without
    /// assistance from the compiler.
    pub unsafe fn destroy(&mut self) {
        ManuallyDrop::drop(&mut self.0)
    }

    /// Returns true if `self` refers to the same thing as `other`
    pub fn is_same_pointer_as(&self, other: &Self) -> bool {
        let self_slots: &T = &self.0;
        let other_slots: &T = &other.0;
        ptr::eq(self_slots, other_slots)
    }
}

impl<'alloc, 'mem, T: ?Sized> Deref for CapCounted<'alloc, 'mem, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'alloc, 'mem, T: ?Sized> Clone for CapCounted<'alloc, 'mem, T> {
    fn clone(&self) -> Self {
        // Safety: This is safe because Cap-Counted ensures that no double-free occurs and the derivation trees cursor
        // ensure aliasing rules
        let (raw_value, raw_allocator, raw_layout) = unsafe { self.0.as_raw() };
        Self(ManuallyDrop::new(Box::from_raw(
            raw_value,
            raw_allocator,
            raw_layout,
        )))
    }
}

impl<'alloc, 'mem, T: ?Sized> From<Box<'alloc, 'mem, T>> for CapCounted<'alloc, 'mem, T> {
    fn from(value: Box<'alloc, 'mem, T>) -> Self {
        CapCounted::from_box(value)
    }
}
