use core::cell::UnsafeCell;
use core::ptr;

pub struct VolatileCell<T> {
    value: UnsafeCell<T>,
}

pub struct Reg<RP, WP, T>
where
    T: Copy,
{
    register: VolatileCell<T>,
    #[allow(dead_code)]
    read: RP,
    #[allow(dead_code)]
    write: WP,
}

pub struct ReadAllowed;

pub struct ReadDenied;

pub struct WriteAllowed;

pub struct WriteDenied;

pub type RW<T> = Reg<ReadAllowed, WriteAllowed, T>;
pub type WO<T> = Reg<ReadDenied, WriteAllowed, T>;
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
    /// # Safety
    /// This function is unsafe, because volatile Cells can have side effects
    #[inline(always)]
    pub unsafe fn get(&self) -> T {
        ptr::read_volatile(self.as_ptr())
    }

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
    /// # Safety
    /// This function is unsafe, because volatile Cells can have side effects
    #[inline(always)]
    pub unsafe fn write(&self, value: T) {
        self.register.set(value)
    }
}

impl<WP, T> Reg<ReadAllowed, WP, T>
where
    T: Copy,
{
    /// # Safety
    /// This function is unsafe, because volatile Cells can have side effects
    #[inline(always)]
    pub unsafe fn read(&self) -> T {
        self.register.get()
    }
}

impl<T> Reg<ReadAllowed, WriteAllowed, T>
where
    T: Copy,
{
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
    #[inline(always)]
    pub fn new(value: T) -> Reg<ReadAllowed, WriteAllowed, T> {
        Self {
            register: VolatileCell::new(value),
            write: WriteAllowed,
            read: ReadAllowed,
        }
    }
}

impl<T> Reg<ReadDenied, WriteAllowed, T>
where
    T: Copy,
{
    #[inline(always)]
    pub fn new(value: T) -> Reg<ReadDenied, WriteAllowed, T> {
        Self {
            register: VolatileCell::new(value),
            write: WriteAllowed,
            read: ReadDenied,
        }
    }
}

impl<T> Reg<ReadAllowed, WriteDenied, T>
where
    T: Copy,
{
    #[inline(always)]
    pub fn new(value: T) -> Reg<ReadAllowed, WriteDenied, T> {
        Self {
            register: VolatileCell::new(value),
            write: WriteDenied,
            read: ReadAllowed,
        }
    }
}
