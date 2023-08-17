/// Used to obtain a `&'static` reference.
///
/// This is similar to [`AsRef`](std::convert::AsRef) except that it hands out `'static` references.
/// There is also the mutable counterpart [`AsStaticMut`] if a mutable reference is desired.
///
/// # Safety
/// This trait is only allowed to be implemented by types that manage the lifetime of `T` through some other means
/// and guarantee that `T` exists for as long as references have been handed out.
pub unsafe trait AsStaticRef<T: ?Sized> {
    /// Converts this type into a shared static reference of the input type.
    fn as_static_ref(&self) -> &'static T;
}

/// Used to obtain a `&'static mut` reference.
///
/// This is similar to [`AsMut`](std::convert::AsMut) except that it hands out `'static` references.
/// There is also the non-mutable counterpart [`AsStaticRef`] if a no mutable reference is needed.
///
/// # Safety
/// This trait is only allowed to be implemented by types that manage the lifetime of `T` through some other means
/// and guarantee that `T` exists for as long as references have been handed out.
pub unsafe trait AsStaticMut<T: ?Sized> {
    fn as_static_mut(&self) -> &'static mut T;
}
