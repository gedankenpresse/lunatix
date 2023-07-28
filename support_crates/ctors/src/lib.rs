//! Advanced constructors and memory semantics
#![no_std]

use core::marker::PhantomData;
use core::mem::MaybeUninit;
use core::ops::{Deref, DerefMut};
use core::pin::Pin;
use core::ptr::drop_in_place;

/// The general trait for Constructors
pub unsafe trait Ctor<T> {
    fn construct(self, dest: Pin<&mut MaybeUninit<T>>);
}

unsafe impl<F, T> Ctor<T> for F
where
    F: FnOnce(Pin<&mut MaybeUninit<T>>),
{
    fn construct(self, dest: Pin<&mut MaybeUninit<T>>) {
        self(dest)
    }
}

pub struct New<F, R>(F, PhantomData<R>);

unsafe impl<F, R> Ctor<R> for New<F, R>
where
    F: FnOnce(Pin<&mut MaybeUninit<R>>),
{
    fn construct(self, dest: Pin<&mut MaybeUninit<R>>) {
        self.0(dest)
    }
}

impl<F, R> New<F, R>
where
    F: FnOnce(Pin<&mut MaybeUninit<R>>),
{
    pub fn ctor(f: F) -> Self {
        Self(f, PhantomData)
    }
}

/// A memory location which is large enough to hold `T`
pub struct OwnedSlot<T> {
    inner: MaybeUninit<T>,
}

impl<T> OwnedSlot<T> {
    pub fn new() -> Self {
        Self {
            inner: MaybeUninit::uninit(),
        }
    }

    pub fn get_slot(self: Pin<&mut Self>) -> Slot<T> {
        Slot {
            inner: unsafe { self.map_unchecked_mut(|s| &mut s.inner) },
        }
    }
}

/// A reference to memory which can be initialized using a constructor.
///
/// It is explicitly useful when requiring the memory to not move as it operates with Pins only.
pub struct Slot<'a, T> {
    inner: Pin<&'a mut MaybeUninit<T>>,
}

impl<'a, T> Slot<'a, T> {
    /// Initialize this memory using the given constructor
    pub fn emplace<C>(self, ctor: C) -> Pin<SlotBox<'a, T>>
    where
        C: Ctor<T>,
    {
        unsafe {
            let ptr = Pin::into_inner_unchecked(self.inner) as *mut MaybeUninit<T>;
            ctor.construct(Pin::new_unchecked(&mut *ptr));
            Pin::new_unchecked(SlotBox {
                inner: &mut *(ptr.cast()),
            })
        }
    }
}

/// A smart pointer to a typically stack owned value.
pub struct SlotBox<'a, T> {
    inner: &'a mut T,
}

impl<'a, T> Deref for SlotBox<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl<'a, T> DerefMut for SlotBox<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner
    }
}

impl<'a, T> Drop for SlotBox<'a, T> {
    fn drop(&mut self) {
        unsafe { drop_in_place(self.inner) }
    }
}

#[macro_export]
macro_rules! slot {
    ($name:ident: $T:ty) => {
        let $name = core::pin::pin!($crate::OwnedSlot::new());
        let $name: $crate::Slot<$T> = $name.get_slot();
    };
    ($name:ident) => {
        let $name = core::pin::pin!($crate::OwnedSlot::new());
        let $name = $name.get_slot();
    };
}

#[macro_export]
macro_rules! emplace {
    ($name:ident = $e:expr) => {
        $crate::slot!($name);
        let $name = $name.emplace($e);
    };
    (mut $name:ident = $e:expr) => {
        $crate::slot!($name);
        let mut $name = $name.emplace($e);
    };
}

#[cfg(test)]
mod test {
    extern crate alloc;
    extern crate std;

    use crate::Ctor;
    use alloc::boxed::Box;
    use core::mem::MaybeUninit;
    use core::pin::Pin;
    use core::sync::atomic::{AtomicUsize, Ordering};

    struct DummyCtor<'a> {
        ctr: &'a AtomicUsize,
    }

    impl<'a> DummyCtor<'a> {
        fn new(ctr: &'static AtomicUsize) -> impl Ctor<DummyCtor<'a>> {
            move |mut dest: Pin<&mut MaybeUninit<DummyCtor>>| {
                ctr.fetch_add(1, Ordering::SeqCst);
                dest.write(DummyCtor { ctr });
            }
        }
    }

    impl<'a> Drop for DummyCtor<'a> {
        fn drop(&mut self) {
            self.ctr.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn test_slot_macro() {
        super::slot!(slot, usize);
    }

    #[test]
    fn test_ctor_calling() {
        super::slot!(slot, DummyCtor);
        let ctr = Box::leak::<'static>(Box::new(AtomicUsize::new(0)));
        let slot_box = slot.emplace(DummyCtor::new(ctr));
        assert_eq!(ctr.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_drop_calling() {
        super::slot!(slot, DummyCtor);
        let ctr = Box::leak::<'static>(Box::new(AtomicUsize::new(0)));
        let slot_box = slot.emplace(DummyCtor::new(ctr));
        drop(slot_box);
        assert_eq!(ctr.load(Ordering::SeqCst), 2)
    }

    #[test]
    fn test_no_double_drop_calling() {
        let ctr = Box::leak::<'static>(Box::new(AtomicUsize::new(0)));
        {
            super::slot!(slot, DummyCtor);
            let slot_box = slot.emplace(DummyCtor::new(ctr));
            drop(slot_box);
            assert_eq!(ctr.load(Ordering::SeqCst), 2)
        }
        assert_eq!(ctr.load(Ordering::SeqCst), 2)
    }
}
