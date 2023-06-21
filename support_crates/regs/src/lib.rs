//! Library for modelling memory mapped registers
#![no_std]

use core::cell::UnsafeCell;
use core::marker::PhantomData;
use core::ptr;

/// A volatile memory area that may change unexpectedly and does not honor normal memory semantics.
///
/// This cell variant always uses explicitly volatile read and write operations that are not optimized out by the
/// compiler.
#[repr(transparent)]
struct VolatileCell<T> {
    value: UnsafeCell<T>,
}

/// A memory mapped register
///
/// This struct is generic over the operations it supports via `ReadOp` and `WriteOp`.
/// These should be [`ReadAllowed`] or [`ReadDenied`] for `ReadOp` and [`WriteAllowed`] or [`WriteDenied`] for `WriteOp`.
pub struct Reg<ReadOp, WriteOp, T>
where
    T: Copy,
{
    register: VolatileCell<T>,
    #[allow(dead_code)]
    read: PhantomData<ReadOp>,
    #[allow(dead_code)]
    write: PhantomData<WriteOp>,
}

/// Marker struct for configuring a [`Reg`] to allow reading from it
pub struct ReadAllowed;

/// Marker struct for configuring a [`Reg`] to deny reading from it
pub struct ReadDenied;

/// Marker struct for configuring a [`Reg`] to allow writing to it
pub struct WriteAllowed;

/// Marker struct for configuring a [`Reg`] to deny writing to it
pub struct WriteDenied;

/// A Register that allows **read and write** interactions
pub type RW<T> = Reg<ReadAllowed, WriteAllowed, T>;

/// A Register that allows **only write** interactions
pub type WO<T> = Reg<ReadDenied, WriteAllowed, T>;

/// A Register that allows **only read** interactions
pub type RO<T> = Reg<ReadAllowed, WriteDenied, T>;

impl<T> VolatileCell<T> {
    pub const fn new(value: T) -> Self {
        Self {
            value: UnsafeCell::new(value),
        }
    }

    #[inline(always)]
    pub fn as_ptr(&self) -> *mut T {
        self.value.get()
    }
}

impl<T> VolatileCell<T>
where
    T: Copy,
{
    /// Read the value contained in the cell
    ///
    /// # Safety
    /// This function is unsafe, because volatile Cells can have side effects
    #[inline(always)]
    pub unsafe fn get(&self) -> T {
        ptr::read_volatile(self.as_ptr())
    }

    /// Set the value contained in the cell
    ///
    /// # Safety
    /// This function is unsafe, because volatile Cells can have side effects
    #[inline(always)]
    pub unsafe fn set(&self, value: T) {
        ptr::write_volatile(self.as_ptr(), value)
    }
}

impl<RP, T> Reg<RP, WriteAllowed, T>
where
    T: Copy,
{
    /// Write a value to the register.
    ///
    /// # Safety
    /// This function is unsafe, because writing to memory mapped registers may have side effects.
    #[inline(always)]
    pub unsafe fn write(&self, value: T) {
        self.register.set(value)
    }
}

impl<WP, T> Reg<ReadAllowed, WP, T>
where
    T: Copy,
{
    /// Read a value from the register.
    ///
    /// # Safety
    /// This function is unsafe, because reading from memory mapped registers may have side effects.
    #[inline(always)]
    pub unsafe fn read(&self) -> T {
        self.register.get()
    }
}

impl<T> Reg<ReadAllowed, WriteAllowed, T>
where
    T: Copy,
{
    /// Modify the value contained in the register by mapping it to another one.
    ///
    /// `f` is called with the current value and should return the new value that will be written back to the register.
    ///
    /// # Safety
    /// This function is unsafe, because volatile Cells can have side effects
    #[inline(always)]
    pub unsafe fn modify<F>(&self, f: F)
    where
        F: FnOnce(T) -> T,
    {
        self.register.set(f(self.register.get()));
    }
}

impl<T> Reg<ReadAllowed, WriteAllowed, T>
where
    T: Copy,
{
    /// Create a new `Reg` pointing to the memory location containing `value`.
    #[inline(always)]
    pub fn new(value: T) -> Reg<ReadAllowed, WriteAllowed, T> {
        Self {
            register: VolatileCell::new(value),
            write: PhantomData::default(),
            read: PhantomData::default(),
        }
    }
}

impl<T> Reg<ReadDenied, WriteAllowed, T>
where
    T: Copy,
{
    /// Create a new `Reg` wrapping the given value.
    #[inline(always)]
    pub fn new(value: T) -> Reg<ReadDenied, WriteAllowed, T> {
        Self {
            register: VolatileCell::new(value),
            write: PhantomData::default(),
            read: PhantomData::default(),
        }
    }
}

impl<T> Reg<ReadAllowed, WriteDenied, T>
where
    T: Copy,
{
    /// Create a new `Reg` pointing to the memory location containing `value`.
    #[inline(always)]
    pub fn new(value: T) -> Reg<ReadAllowed, WriteDenied, T> {
        Self {
            register: VolatileCell::new(value),
            write: PhantomData::default(),
            read: PhantomData::default(),
        }
    }
}
