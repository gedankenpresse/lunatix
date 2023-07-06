use crate::bump_allocator::bump_alloc_trait::BumpAllocator;
use crate::{AllocFailed, AllocInit};
use core::mem;
use core::mem::MaybeUninit;
use core::ops::{Deref, DerefMut};
use core::ptr;

/// A box-like struct allocated from a [`BumpAllocator`]
pub struct BumpBox<'alloc, 'mem, A: BumpAllocator<'mem>, T: ?Sized> {
    inner: &'mem mut T,
    source: &'alloc A,
}

impl<'alloc, 'mem, A: BumpAllocator<'mem>, T> BumpBox<'alloc, 'mem, A, T> {
    /// Allocate memory from the given allocator and store the given data in it.
    pub fn new(data: T, allocator: &'alloc A) -> Result<Self, AllocFailed> {
        let result = Self::new_uninit(allocator)?;
        Ok(unsafe {
            result.inner.as_mut_ptr().cast::<T>().write(data);
            result.assume_init()
        })
    }

    /// Construct a new box with uninitialized content
    pub fn new_uninit(
        allocator: &'alloc A,
    ) -> Result<BumpBox<'alloc, 'mem, A, MaybeUninit<T>>, AllocFailed> {
        // allocate enough space from the allocator
        let mem = allocator
            .allocate(
                mem::size_of::<T>(),
                mem::align_of::<T>(),
                AllocInit::Uninitialized,
            )?
            .as_mut_ptr()
            .cast::<MaybeUninit<T>>();

        // put the given data into the allocated memory
        Ok(unsafe {
            BumpBox {
                inner: &mut *mem,
                source: allocator,
            }
        })
    }

    /// Construct a new box with uninitialized content but the underlying memory being filled with `0` bytes.
    ///
    /// See [`MaybeUninit::zeroed`] for examples of correct and incorrect usage of this method.
    pub fn new_zeroed(
        allocator: &'alloc A,
    ) -> Result<BumpBox<'alloc, 'mem, A, MaybeUninit<T>>, AllocFailed> {
        // allocate enough space from the allocator
        let mem = allocator
            .allocate(mem::size_of::<T>(), mem::align_of::<T>(), AllocInit::Zeroed)?
            .as_mut_ptr()
            .cast::<MaybeUninit<T>>();

        // put the given data into the allocated memory
        Ok(unsafe {
            BumpBox {
                inner: &mut *mem,
                source: allocator,
            }
        })
    }
}

impl<'alloc, 'mem, A: BumpAllocator<'mem>, T> BumpBox<'alloc, 'mem, A, [T]> {
    /// Constructs a new boxed slice with uninitialized contents.
    pub fn new_uninit_slice(
        len: usize,
        allocator: &'alloc A,
    ) -> Result<BumpBox<'alloc, 'mem, A, [MaybeUninit<T>]>, AllocFailed> {
        Self::new_uninit_slice_with_alignment(len, mem::align_of::<T>(), allocator)
    }

    /// Construct a new boxed slice with uninitialized contents.
    ///
    /// The slice is guaranteed to start at an explicitly aligned address.
    pub fn new_uninit_slice_with_alignment(
        len: usize,
        alignment: usize,
        allocator: &'alloc A,
    ) -> Result<BumpBox<'alloc, 'mem, A, [MaybeUninit<T>]>, AllocFailed> {
        // allocate enough space from the allocator
        let mem = ptr::slice_from_raw_parts_mut(
            allocator
                .allocate(
                    mem::size_of::<T>() * len,
                    alignment,
                    AllocInit::Uninitialized,
                )?
                .as_mut_ptr()
                .cast::<MaybeUninit<T>>(),
            len,
        );

        // put the given data into the allocated memory
        Ok(unsafe {
            BumpBox {
                inner: &mut *mem,
                source: allocator,
            }
        })
    }
}

impl<'alloc, 'mem, A: BumpAllocator<'mem>, T: ?Sized> BumpBox<'alloc, 'mem, A, T> {
    /// Consumes the Box, returning a raw pointer to the underlying data.
    ///
    /// After calling this function, the caller is responsible for the memory previously managed by the Box.
    /// In particular, the caller should properly destroy T and release the memory back to the allocator.
    pub fn into_raw(self) -> *mut T {
        let result = self.inner as *mut T;
        mem::forget(self);
        result
    }

    /// Consumes the box and leaks the contained value, returning a mutable reference to it.
    ///
    /// Note that dropping the returned reference will produce a memory leak.
    pub fn leak(self) -> &'mem mut T {
        let value_ptr = self.inner as *mut T;
        mem::forget(self);
        unsafe { &mut *value_ptr }
    }
}

impl<'alloc, 'mem, A: BumpAllocator<'mem>, T> BumpBox<'alloc, 'mem, A, MaybeUninit<T>> {
    /// Converts to `BumpBox<T>`
    ///
    /// # Safety
    /// As with [`MaybeUninit::assume_init`], it is up to the caller to guarantee that the value really is in an initialized state.
    /// Calling this when the content is not yet fully initialized causes immediate undefined behavior.
    pub unsafe fn assume_init(self) -> BumpBox<'alloc, 'mem, A, T> {
        let mut old = mem::ManuallyDrop::new(self);
        BumpBox {
            inner: &mut *old.inner.as_mut_ptr().cast(),
            source: old.source,
        }
    }
}

impl<'alloc, 'mem, A: BumpAllocator<'mem>, T> BumpBox<'alloc, 'mem, A, [MaybeUninit<T>]> {
    /// Converts to `BumpBox<T>`
    ///
    /// # Safety
    /// As with [`MaybeUninit::assume_init`], it is up to the caller to guarantee that the value really is in an initialized state.
    /// Calling this when the content is not yet fully initialized causes immediate undefined behavior.
    pub unsafe fn assume_init(self) -> BumpBox<'alloc, 'mem, A, [T]> {
        let mut old = mem::ManuallyDrop::new(self);
        BumpBox {
            inner: &mut *ptr::slice_from_raw_parts_mut(
                old.inner.as_mut_ptr() as *mut T,
                old.inner.len(),
            ),
            source: old.source,
        }
    }
}

impl<'mem, A: BumpAllocator<'mem>, T: ?Sized> Deref for BumpBox<'_, 'mem, A, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl<'mem, A: BumpAllocator<'mem>, T: ?Sized> DerefMut for BumpBox<'_, 'mem, A, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner
    }
}

impl<'mem, A: BumpAllocator<'mem>, T: ?Sized> Drop for BumpBox<'_, 'mem, A, T> {
    fn drop(&mut self) {
        unsafe { self.source.deallocate(self.inner as *mut T as *mut u8) }
    }
}
