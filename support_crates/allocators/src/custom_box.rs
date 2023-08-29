use crate::traits::{AllocError, AllocInit, Allocator};
use core::alloc::Layout;
use core::fmt::{Display, Formatter};
use core::mem;
use core::mem::MaybeUninit;
use core::ops::{Deref, DerefMut};
use core::pin::Pin;
use core::ptr;

/// A custom box implementation based on our own allocator implementation
///
/// # Generic Argumentsy
/// - `'alloc` is the lifetime of the allocator from which the underlying memory was borrowed.
/// - `'mem` is the lifetime of the underlying memory from which the source allocator allocates.
/// - `A` is the [`Allocator`] implementation.
pub struct Box<'alloc, 'mem, T: ?Sized> {
    /// The heap-allocated value that is managed by this box
    inner: &'mem mut T,
    /// The allocator from which the backing memory was taken
    source_alloc: &'alloc dyn Allocator<'mem>,
    /// The layout request that was used during the allocation of the backing memory
    source_layout: Layout,
}

// general maybe-sized impl

impl<'alloc, 'mem, T: ?Sized> Box<'alloc, 'mem, T> {
    /// Consume the Box and leak the held value.
    ///
    /// This function is mainly useful for data that lives for the remainder of the program's life.
    /// Dropping the returned reference will cause a memory leak.
    /// If this is not acceptable, us [`into_raw()`](Box::into_raw) and [`from_raw()`](Box::from_raw) instead.
    pub fn leak(self) -> &'mem mut T {
        let result = self.inner as *mut T;
        mem::forget(self);
        unsafe { &mut *result }
    }

    pub unsafe fn ignore_lifetimes(self) -> Box<'static, 'mem, T> {
        unsafe { core::mem::transmute::<Box<'alloc, 'mem, T>, Box<'static, 'mem, T>>(self) }
    }

    /// Consume the Box, returning its raw parts.
    ///
    /// After calling this function, the caller is responsible for the memory previously managed by the Box.
    /// In particular, the caller should properly destroy `T` and release the memory back to the allocator.
    ///
    /// The easiest way to to this is to construct another box using [`from_raw()`](Box::from_raw) and then dropping
    /// it.
    pub fn into_raw(self) -> (&'mem mut T, &'alloc dyn Allocator<'mem>, Layout) {
        let result = (
            unsafe { &mut *(self.inner as *mut T) },
            self.source_alloc,
            self.source_layout,
        );
        mem::forget(self);
        result
    }

    /// Return the raw parts of the box.
    ///
    /// # Safety
    /// Since the box is not consumed by this function, it is up to the caller to ensure that no aliasing errors are
    /// created when using the contained memory and that no double-free occurs.
    pub unsafe fn as_raw(&self) -> (&'mem mut T, &'alloc dyn Allocator<'mem>, Layout) {
        unsafe {
            (
                #[warn(cast_ref_to_mut)]
                &mut *(self.inner as *const _ as *mut _),
                &*(self.source_alloc as *const _),
                self.source_layout,
            )
        }
    }

    /// Construct a box from raw data in the given allocator.
    ///
    /// After calling this function, the data is owned by the resulting box.
    /// Specifically, the `Box` destructor will call the destructor of `T` and free the allocated memory.
    pub fn from_raw(
        data: &'mem mut T,
        source_allocator: &'alloc dyn Allocator<'mem>,
        source_layout: Layout,
    ) -> Self {
        Self {
            inner: data,
            source_alloc: source_allocator,
            source_layout,
        }
    }

    /// Converts a `Box<T>` into a `Pin<Box<T>>`.
    /// If `T` does not implement [`Unpin`], then `*self` will be pinned in memory and unable to be moved.
    ///
    /// This conversion does not allocate again and happens in place.
    ///
    /// This is also available via [`From`].
    pub fn into_pin(self) -> Pin<Box<'alloc, 'mem, T>> {
        // It's not possible to move or replace the insides of a `Pin<Box<T>>`
        // when `T: !Unpin`, so it's safe to pin it directly without any
        // additional requirements.
        unsafe { Pin::new_unchecked(self) }
    }
}

// general sized impl

impl<'alloc, 'mem, T: Sized> Box<'alloc, 'mem, T> {
    /// Store the given value on the heap by allocating memory from an allocator and using that to
    /// store it.
    pub fn new(value: T, allocator: &'alloc dyn Allocator<'mem>) -> Result<Self, AllocError> {
        let result = Self::new_raw(allocator, Layout::new::<T>(), AllocInit::Uninitialized)?;
        Ok(unsafe {
            result.inner.as_mut_ptr().cast::<T>().write(value);
            result.assume_init()
        })
    }

    /// Construct a new `Pin<Box<T>>`.
    /// If `T` does not implement [`Unpin`], then `value` will
    pub fn new_pinned(
        value: T,
        allocator: &'alloc dyn Allocator<'mem>,
    ) -> Result<Pin<Self>, AllocError> {
        Ok(Self::new(value, allocator)?.into_pin())
    }

    /// Construct a new Box able to hold `T` but with uninitialized content
    pub fn new_uninit(
        allocator: &'alloc dyn Allocator<'mem>,
    ) -> Result<Box<'alloc, 'mem, MaybeUninit<T>>, AllocError> {
        Self::new_raw(allocator, Layout::new::<T>(), AllocInit::Uninitialized)
    }

    /// Construct a new Box able to hold `T` but with zero-initialized content.
    ///
    /// See [`MaybeUninit::zeroed`] for examples of correct and incorrect usage of this method
    /// but generally it depends on `T` whether or not memory filled with `0` bytes can be considered
    /// valid or not.
    pub fn new_zeroed(
        allocator: &'alloc dyn Allocator<'mem>,
    ) -> Result<Box<'alloc, 'mem, MaybeUninit<T>>, AllocError> {
        Box::new_raw(allocator, Layout::new::<T>(), AllocInit::Zeroed)
    }

    fn new_raw(
        allocator: &'alloc dyn Allocator<'mem>,
        layout: Layout,
        alloc_init: AllocInit,
    ) -> Result<Box<'alloc, 'mem, MaybeUninit<T>>, AllocError> {
        let mem = allocator
            .allocate(layout, alloc_init)?
            .as_mut_ptr()
            .cast::<MaybeUninit<T>>();

        Ok(Box {
            inner: unsafe { &mut *mem },
            source_alloc: allocator,
            source_layout: layout,
        })
    }
}

// general slice impls

impl<'alloc, 'mem, T> Box<'alloc, 'mem, [T]> {
    /// Create a new boxed slice with uninitialized contents.
    pub fn new_uninit_slice(
        len: usize,
        allocator: &'alloc dyn Allocator<'mem>,
    ) -> Result<Box<'alloc, 'mem, [MaybeUninit<T>]>, AllocError> {
        Self::new_slice_raw(
            len,
            allocator,
            Layout::array::<T>(len).map_err(|_| AllocError::InsufficientMemory)?,
            AllocInit::Uninitialized,
        )
    }

    /// Create a new boxed slice with zero-initialized contents.
    ///
    /// See [`MaybeUninit::zeroed`] for examples of correct and incorrect usage of this method
    /// but generally it depends on `T` whether or not memory filled with `0` bytes can be considered
    /// valid or not.
    pub fn new_zeroed_slice(
        len: usize,
        allocator: &'alloc dyn Allocator<'mem>,
    ) -> Result<Box<'alloc, 'mem, [MaybeUninit<T>]>, AllocError> {
        Self::new_slice_raw(
            len,
            allocator,
            Layout::array::<T>(len).map_err(|_| AllocError::InsufficientMemory)?,
            AllocInit::Zeroed,
        )
    }

    /// Create a new boxed slice with uninitialized contents and an explicit start alignment.
    pub fn new_uninit_slice_with_alignment(
        len: usize,
        alignment: usize,
        allocator: &'alloc dyn Allocator<'mem>,
    ) -> Result<Box<'alloc, 'mem, [MaybeUninit<T>]>, AllocError> {
        Self::new_slice_raw(
            len,
            allocator,
            Layout::array::<T>(len).and_then(|layout| layout.align_to(alignment))?,
            AllocInit::Zeroed,
        )
    }

    fn new_slice_raw(
        len: usize,
        allocator: &'alloc dyn Allocator<'mem>,
        layout: Layout,
        alloc_init: AllocInit,
    ) -> Result<Box<'alloc, 'mem, [MaybeUninit<T>]>, AllocError> {
        let mem = allocator
            .allocate(layout, alloc_init)?
            .as_mut_ptr()
            .cast::<MaybeUninit<T>>();
        let mem = ptr::slice_from_raw_parts_mut(mem, len);

        Ok(Box {
            inner: unsafe { &mut *mem },
            source_alloc: allocator,
            source_layout: layout,
        })
    }
}

// impls for calling assume_init()

impl<'alloc, 'mem, T> Box<'alloc, 'mem, MaybeUninit<T>> {
    /// Converts to `Box<T>`
    ///
    /// # Safety
    /// As with [`MaybeUninit::assume_init`], it is up to the caller to guarantee that the value really is in an initialized state.
    /// Calling this when the content is not yet fully initialized causes immediate undefined behavior.
    pub unsafe fn assume_init(self) -> Box<'alloc, 'mem, T> {
        // prevent drop() being called which would deallocate the memory
        let mut old = mem::ManuallyDrop::new(self);

        Box {
            inner: &mut *old.inner.as_mut_ptr().cast(),
            source_alloc: old.source_alloc,
            source_layout: old.source_layout,
        }
    }
}

impl<'alloc, 'mem, T> Box<'alloc, 'mem, [MaybeUninit<T>]> {
    /// Converts to `Box<T>`
    ///
    /// # Safety
    /// As with [`MaybeUninit::assume_init`], it is up to the caller to guarantee that the value really is in an initialized state.
    /// Calling this when the content is not yet fully initialized causes immediate undefined behavior.
    pub unsafe fn assume_init(self) -> Box<'alloc, 'mem, [T]> {
        // prevent drop() being called which would deallocate the memory
        let mut old = mem::ManuallyDrop::new(self);

        Box {
            inner: &mut *ptr::slice_from_raw_parts_mut(
                old.inner.as_mut_ptr().cast(),
                old.inner.len(),
            ),
            source_alloc: old.source_alloc,
            source_layout: old.source_layout,
        }
    }
}

// Drop impl

impl<'alloc, 'mem, T: ?Sized> Drop for Box<'alloc, 'mem, T> {
    fn drop(&mut self) {
        unsafe {
            self.source_alloc
                .deallocate(self.inner as *mut T as *mut u8, self.source_layout)
        }
    }
}

// Deref and DerefMut impls

impl<'alloc, 'mem, T: ?Sized> Deref for Box<'alloc, 'mem, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl<'alloc, 'mem, T: ?Sized> DerefMut for Box<'alloc, 'mem, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner
    }
}

// Display impl

impl<'alloc, 'mem, T: ?Sized + Display> Display for Box<'alloc, 'mem, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        self.inner.fmt(f)
    }
}

// Pin construction impl

impl<'alloc, 'mem, T: ?Sized> From<Box<'alloc, 'mem, T>> for Pin<Box<'alloc, 'mem, T>> {
    fn from(value: Box<'alloc, 'mem, T>) -> Self {
        value.into_pin()
    }
}
