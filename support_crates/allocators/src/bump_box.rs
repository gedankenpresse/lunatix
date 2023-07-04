use crate::{AllocError, AllocInit, BumpAllocator};
use core::marker::PhantomData;
use core::mem;
use core::mem::MaybeUninit;
use core::ops::{Deref, DerefMut};
use core::ptr;

/// A box-like struct allocated from a [`BumpAllocator`]
pub struct BumpBox<'alloc, 'mem, T: ?Sized> {
    inner: &'mem mut T,
    source: &'alloc BumpAllocator<'mem>,
}

impl<'alloc, 'mem, T> BumpBox<'alloc, 'mem, T> {
    /// Allocate memory from the given allocator and store the given data in it.
    pub fn new(data: T, allocator: &'alloc BumpAllocator<'mem>) -> Result<Self, AllocError> {
        let result = Self::new_uninit(allocator)?;
        Ok(unsafe {
            result.inner.as_mut_ptr().cast::<T>().write(data);
            result.assume_init()
        })
    }

    /// Construct a new box with uninitialized content
    pub fn new_uninit(
        allocator: &'alloc BumpAllocator<'mem>,
    ) -> Result<BumpBox<'alloc, 'mem, MaybeUninit<T>>, AllocError> {
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
}

impl<'alloc, 'mem, T> BumpBox<'alloc, 'mem, [T]> {
    /// Constructs a new boxed slice with uninitialized contents.
    pub fn new_uninit_slice(
        len: usize,
        allocator: &'alloc BumpAllocator<'mem>,
    ) -> Result<BumpBox<'alloc, 'mem, [MaybeUninit<T>]>, AllocError> {
        // allocate enough space from the allocator
        let mem = ptr::slice_from_raw_parts_mut(
            allocator
                .allocate(
                    mem::size_of::<T>() * len,
                    mem::align_of::<T>(),
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

impl<'alloc, 'mem, T: ?Sized> BumpBox<'alloc, 'mem, T> {
    pub fn into_raw(self) -> *mut T {
        self.inner as *mut T
    }
}

impl<'alloc, 'mem, T> BumpBox<'alloc, 'mem, MaybeUninit<T>> {
    /// Converts to `BumpBox<T>`
    ///
    /// # Safety
    /// As with [`MaybeUninit::assume_init`], it is up to the caller to guarantee that the value really is in an initialized state.
    /// Calling this when the content is not yet fully initialized causes immediate undefined behavior.
    pub unsafe fn assume_init(self) -> BumpBox<'alloc, 'mem, T> {
        BumpBox {
            inner: &mut *self.inner.as_mut_ptr().cast(),
            source: self.source,
        }
    }
}

impl<'alloc, 'mem, T> BumpBox<'alloc, 'mem, [MaybeUninit<T>]> {
    /// Converts to `BumpBox<T>`
    ///
    /// # Safety
    /// As with [`MaybeUninit::assume_init`], it is up to the caller to guarantee that the value really is in an initialized state.
    /// Calling this when the content is not yet fully initialized causes immediate undefined behavior.
    pub unsafe fn assume_init(self) -> BumpBox<'alloc, 'mem, [T]> {
        BumpBox {
            inner: &mut *ptr::slice_from_raw_parts_mut(
                self.inner.as_mut_ptr() as *mut T,
                self.inner.len(),
            ),
            source: self.source,
        }
    }
}

impl<T: ?Sized> Deref for BumpBox<'_, '_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl<T: ?Sized> DerefMut for BumpBox<'_, '_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner
    }
}

impl<T: ?Sized> Drop for BumpBox<'_, '_, T> {
    fn drop(&mut self) {
        unsafe { self.source.deallocate(self.inner as *mut T as *mut u8) }
    }
}
