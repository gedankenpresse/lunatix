//! Additional pointer types with explicit address space semantics
//!
//! These types don't have a large functional purpose but instead they exist to make APIs which deal with virtual
//! and physical addresses more clear and type-safe.
//! The goal is to make it clear in a functions API what type of address is expected and thus document how it will be
//! treated by the function.

use crate::mem::{VIRT_MEM_PHYS_MAP_END, VIRT_MEM_PHYS_MAP_START};
use core::fmt::Formatter;
use core::{fmt, ptr};

/// A const-pointer with physical addressing semantics.
///
/// This pointer is used to hold physical addresses.
/// That is addresses which are resolvable by the CPU when virtual addressing is turned **off**.
#[derive(Debug)]
#[repr(transparent)]
pub struct PhysConstPtr<T>(*const T);

/// A mut-pointer with physical addressing semantics.
///
/// This pointer is used to hold physical addresses.
/// That is addresses which are resolvable by the CPU when virtual addressing is turned **off**.
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct PhysMutPtr<T>(*mut T);

/// A const-pointer with virtual addressing semantics.
///
/// This pointer is used to hold virtual addresses.
/// That is addresses which a resolvable by the CPU when virtual addressing is turned **on** and when a
/// [`PageTable`](super::PageTable) has been configured to resolve it.
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct VirtConstPtr<T>(*const T);

/// A mut-pointer with virtual addressing semantics.
///
/// This pointer is used to hold virtual addresses.
/// That is addresses which a resolvable by the CPU when virtual addressing is turned **on** and when a
/// [`PageTable`](super::PageTable) has been configured to resolve it.
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct VirtMutPtr<T>(*mut T);

/// A const-pointer which points to a memory location that has been explicitly mapped to allow direct access to
/// arbitrary locations in physical memory.
///
/// See the [`mem` module docs](super) for more information about the memory area this pointer refers to.
///
/// Essentially this pointer *is* a pointer to a virtual address but it holds special semantic value.
/// As such, it is only resolvable by the CPU when virtual addressing is turned **on** and when a
/// [`PageTable`](super::PageTable) has been configured to resolve it.
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct MappedConstPtr<T>(*const T);

/// A mut-pointer which points to a memory location that has been explicitly mapped to allow direct access to
/// arbitrary locations in physical memory.
///
/// See the [`mem` module docs](super) for more information about the memory area this pointer refers to.
///
/// Essentially this pointer *is* a pointer to a virtual address but it holds special semantic value.
/// As such, it is only resolvable by the CPU when virtual addressing is turned **on** and when a
/// [`PageTable`](super::PageTable) has been configured to resolve it.
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct MappedMutPtr<T>(*mut T);

// PhysConstPtr impls

impl<T> PhysConstPtr<T> {
    /// Calculate where in virtual memory the memory of this pointer is mapped
    pub fn as_mapped(self) -> MappedConstPtr<T> {
        if self.0 as usize == 0 {
            ptr::null::<T>().into()
        } else {
            assert!((self.0.cast::<u8>() as usize) < VIRT_MEM_PHYS_MAP_END - VIRT_MEM_PHYS_MAP_START, "physical address {:p} is larger than the maximum supported directly mapped address", self);
            unsafe { self.0.cast::<u8>().add(VIRT_MEM_PHYS_MAP_START).cast::<T>() }.into()
        }
    }

    /// Deconstruct self into the raw contained pointer
    pub fn raw(self) -> *const T {
        self.0
    }

    /// Create a null pointer
    pub const fn null() -> Self {
        Self(0 as *const T)
    }
}

impl<T> From<*const T> for PhysConstPtr<T> {
    fn from(value: *const T) -> Self {
        Self(value)
    }
}

impl<T> From<PhysConstPtr<T>> for *const T {
    fn from(value: PhysConstPtr<T>) -> Self {
        value.0
    }
}

impl<T> fmt::Pointer for PhysConstPtr<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let x: *const T = self.0;
        f.write_fmt(format_args!("{:#x}", x as usize))
    }
}

impl<T> Clone for PhysConstPtr<T> {
    fn clone(&self) -> Self {
        Self(self.0)
    }
}
impl<T> Copy for PhysConstPtr<T> {}

// PhysMutPtr impls

impl<T> PhysMutPtr<T> {
    /// Calculate where in virtual memory the memory of this pointer is mapped
    pub fn as_mapped(self) -> MappedMutPtr<T> {
        if self.0 as usize == 0 {
            ptr::null_mut::<T>().into()
        } else {
            assert!((self.0.cast::<u8>() as usize) < VIRT_MEM_PHYS_MAP_END - VIRT_MEM_PHYS_MAP_START, "physical address {:p} is larger than the maximum supported directly mapped address", self);
            unsafe { self.0.cast::<u8>().add(VIRT_MEM_PHYS_MAP_START).cast::<T>() }.into()
        }
    }

    /// Deconstruct self into the raw contained pointer
    pub fn raw(self) -> *mut T {
        self.0
    }

    /// Create a null pointer
    pub const fn null() -> Self {
        Self(0 as *mut T)
    }
}

impl<T> From<*mut T> for PhysMutPtr<T> {
    fn from(value: *mut T) -> Self {
        Self(value)
    }
}

impl<T> From<PhysMutPtr<T>> for *mut T {
    fn from(value: PhysMutPtr<T>) -> Self {
        value.0
    }
}

impl<T> fmt::Pointer for PhysMutPtr<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("{:p}", self.0))
    }
}

// VirtConstPtr impls

impl<T> VirtConstPtr<T> {
    // TODO Implement resolution to PhysConstPtr via pagetable walk

    /// Deconstruct self into the raw contained pointer
    pub fn raw(self) -> *const T {
        self.0
    }

    /// Create a null pointer
    pub const fn null() -> Self {
        Self(0 as *const T)
    }
}

impl<T> From<*const T> for VirtConstPtr<T> {
    fn from(value: *const T) -> Self {
        Self(value)
    }
}

impl<T> From<VirtConstPtr<T>> for *const T {
    fn from(value: VirtConstPtr<T>) -> Self {
        value.0
    }
}

impl<T> From<MappedConstPtr<T>> for VirtConstPtr<T> {
    fn from(value: MappedConstPtr<T>) -> Self {
        // mapped addresses *are* virtual addresses so this is easily possible
        Self(value.0)
    }
}

impl<T> fmt::Pointer for VirtConstPtr<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("{:p}", self.0))
    }
}

// VirtMutPtr impls

impl<T> VirtMutPtr<T> {
    // TODO Implement resolution to PhysMutPtr via pagetable walk

    /// Deconstruct self into the raw contained pointer
    pub fn raw(self) -> *mut T {
        self.0
    }

    /// Create a null pointer
    pub const fn null() -> Self {
        Self(0 as *mut T)
    }
}

impl<T> From<*mut T> for VirtMutPtr<T> {
    fn from(value: *mut T) -> Self {
        Self(value)
    }
}

impl<T> From<VirtMutPtr<T>> for *mut T {
    fn from(value: VirtMutPtr<T>) -> Self {
        value.0
    }
}

impl<T> From<MappedMutPtr<T>> for VirtMutPtr<T> {
    fn from(value: MappedMutPtr<T>) -> Self {
        // mapped addresses *are* virtual addresses so this is easily possible
        Self(value.0)
    }
}

impl<T> fmt::Pointer for VirtMutPtr<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("{:p}", self.0))
    }
}

// MappedConstPtr impls

impl<T> MappedConstPtr<T> {
    /// Calculate where in physical memory this mapped memory lies
    pub fn as_direct(self) -> PhysConstPtr<T> {
        if self.0 as usize == 0 {
            ptr::null::<T>().into()
        } else {
            assert!(
                self.0 as usize >= VIRT_MEM_PHYS_MAP_START
                    && self.0 as usize <= VIRT_MEM_PHYS_MAP_END,
                "{:p} does not lie in mapped physical memory",
                self
            );

            unsafe { self.0.cast::<u8>().sub(VIRT_MEM_PHYS_MAP_START).cast::<T>() }.into()
        }
    }

    /// Deconstruct self into the raw contained pointer
    pub fn raw(self) -> *const T {
        self.0
    }

    /// Create a null pointer
    pub const fn null() -> Self {
        Self(0 as *const T)
    }
}

impl<T> From<*const T> for MappedConstPtr<T> {
    fn from(value: *const T) -> Self {
        assert!(
            value as usize >= VIRT_MEM_PHYS_MAP_START && value as usize <= VIRT_MEM_PHYS_MAP_END,
            "{:p} does not lie in mapped physical memory",
            value
        );
        Self(value)
    }
}

impl<T> From<MappedConstPtr<T>> for *const T {
    fn from(value: MappedConstPtr<T>) -> Self {
        value.0
    }
}

impl<T> From<VirtConstPtr<T>> for MappedConstPtr<T> {
    fn from(value: VirtConstPtr<T>) -> Self {
        // mapped addresses *are* virtual addresses so as long as the address is in the mapped memory region,
        // conversion is easily possible
        assert!(
            value.0 as usize >= VIRT_MEM_PHYS_MAP_START
                && value.0 as usize <= VIRT_MEM_PHYS_MAP_END,
            "{:p} does not lie in mapped physical memory",
            value
        );
        Self(value.0)
    }
}

impl<T> fmt::Pointer for MappedConstPtr<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("{:p}", self.0))
    }
}

// MappedMutPtr impls

impl<T> MappedMutPtr<T> {
    /// Calculate where in physical memory this mapped memory lies
    pub fn as_direct(self) -> PhysMutPtr<T> {
        if self.0 as usize == 0 {
            ptr::null_mut::<T>().into()
        } else {
            assert!(
                self.0 as usize >= VIRT_MEM_PHYS_MAP_START
                    && self.0 as usize <= VIRT_MEM_PHYS_MAP_END,
                "{:p} does not lie in mapped physical memory",
                self
            );

            unsafe { self.0.cast::<u8>().sub(VIRT_MEM_PHYS_MAP_START).cast::<T>() }.into()
        }
    }

    /// Deconstruct self into the raw contained pointer
    pub fn raw(self) -> *mut T {
        self.0
    }

    /// Create a null pointer
    pub const fn null() -> Self {
        Self(0 as *mut T)
    }
}

impl<T> From<*mut T> for MappedMutPtr<T> {
    fn from(value: *mut T) -> Self {
        assert!(
            value as usize >= VIRT_MEM_PHYS_MAP_START && value as usize <= VIRT_MEM_PHYS_MAP_END,
            "{:p} does not lie in mapped physical memory",
            value
        );
        Self(value)
    }
}

impl<T> From<MappedMutPtr<T>> for *mut T {
    fn from(value: MappedMutPtr<T>) -> Self {
        value.0
    }
}

impl<T> From<VirtMutPtr<T>> for MappedMutPtr<T> {
    fn from(value: VirtMutPtr<T>) -> Self {
        // mapped addresses *are* virtual addresses so as long as the address is in the mapped memory region,
        // conversion is easily possible
        assert!(
            value.0 as usize >= VIRT_MEM_PHYS_MAP_START
                && value.0 as usize <= VIRT_MEM_PHYS_MAP_END,
            "{:p} does not lie in mapped physical memory",
            value
        );
        Self(value.0)
    }
}

impl<T> fmt::Pointer for MappedMutPtr<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("{:p}", self.0))
    }
}
