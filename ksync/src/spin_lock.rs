use core::cell::UnsafeCell;
use core::hint::spin_loop;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicBool, Ordering};

/// A SpinLock Mutex implementation
#[derive(Debug)]
pub struct SpinLock<T> {
    is_locked: AtomicBool,
    value: UnsafeCell<T>,
}

/// A Guard protecting some data locked through a [`SpinLock`].
///
/// Use it via the implemented [`Deref`] and [`DerefMut`] traits.
pub struct Guard<'a, T> {
    lock: &'a SpinLock<T>,
}

impl<T> SpinLock<T> {
    pub const fn new(value: T) -> Self {
        Self {
            is_locked: AtomicBool::new(false),
            value: UnsafeCell::new(value),
        }
    }

    /// Try to lock the lock and return a [`Guard`] if successful
    pub fn try_lock(&self) -> Result<Guard<T>, ()> {
        if self.is_locked.swap(true, Ordering::Acquire) {
            Err(())
        } else {
            Ok(Guard { lock: self })
        }
    }

    /// Try to repeatedly lock the lock until it succeeds, returning the protected data via a [`Guard`]
    pub fn spin_lock(&self) -> Guard<T> {
        while self.is_locked.swap(true, Ordering::Acquire) {
            spin_loop();
        }
        Guard { lock: self }
    }

    fn unlock(&self) {
        self.is_locked.store(false, Ordering::Release)
    }
}

unsafe impl<T> Sync for SpinLock<T> where T: Send {}

impl<T> Deref for Guard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // Safety: The very existence of this Guard guarantees we've exclusively locked the lock
        unsafe { &*self.lock.value.get() }
    }
}

impl<T> DerefMut for Guard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // Safety: The very existence of this Guard guarantees we've exclusively locked the lock
        unsafe { &mut *self.lock.value.get() }
    }
}

impl<T> Drop for Guard<'_, T> {
    fn drop(&mut self) {
        self.lock.unlock();
    }
}
