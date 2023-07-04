use core::{
    fmt::Pointer,
    mem,
    ops::{Deref, DerefMut},
};

pub type Page = [u8; 4096];
pub const PAGESIZE: usize = core::mem::size_of::<Page>();

const GB: usize = 1024 * 1024 * 1024;
// only valid with rv39
const KERNEL_BASE: usize = !(256 * GB - 1);

#[repr(transparent)]
#[derive(Debug)]
pub struct PhysConstPtr<T>(pub *const T);

impl<T> Copy for PhysConstPtr<T> {}
impl<T> Clone for PhysConstPtr<T> {
    fn clone(&self) -> Self {
        PhysConstPtr(self.0)
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct PhysMutPtr<T>(pub *mut T);

impl<T> Copy for PhysMutPtr<T> {}
impl<T> Clone for PhysMutPtr<T> {
    fn clone(&self) -> Self {
        PhysMutPtr(self.0)
    }
}

#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct PhysRef<'a, T>(&'a T);

#[repr(transparent)]
pub struct PhysMutRef<'a, T>(&'a mut T);

impl<T> Pointer for PhysConstPtr<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.fmt(f)
    }
}

impl<'a, T> Deref for PhysRef<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        phys_to_kernel(PhysRef(self.0))
    }
}

impl<'a, T> Deref for PhysMutRef<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        phys_to_kernel(PhysRef(self.0 as &T))
    }
}

impl<'a, T> DerefMut for PhysMutRef<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        phys_to_kernel_mut(PhysMutRef(self.0))
    }
}

pub fn phys_to_kernel_usize(addr: usize) -> usize {
    assert!(addr < (64 * GB));
    if addr == 0 {
        return 0;
    }
    return addr + KERNEL_BASE;
}

pub fn kernel_to_phys_usize(addr: usize) -> usize {
    assert!(addr >= KERNEL_BASE);
    let phys = addr - KERNEL_BASE;
    assert!(phys < (64 * GB));
    return phys;
}

pub fn phys_to_kernel_mut_ptr<T>(ptr: PhysMutPtr<T>) -> *mut T {
    phys_to_kernel_usize(ptr.0 as usize) as *mut T
}

pub fn kernel_to_phys_mut_ptr<T>(ptr: *mut T) -> PhysMutPtr<T> {
    PhysMutPtr(kernel_to_phys_usize(ptr as usize) as *mut T)
}

pub fn phys_to_kernel_ptr<T>(ptr: PhysConstPtr<T>) -> *const T {
    phys_to_kernel_usize(ptr.0 as usize) as *const T
}

pub fn kernel_to_phys_ptr<T>(ptr: *const T) -> PhysConstPtr<T> {
    PhysConstPtr(kernel_to_phys_usize(ptr as usize) as *const T)
}

pub fn phys_to_kernel<T>(v: PhysRef<T>) -> &T {
    unsafe { core::mem::transmute(phys_to_kernel_usize(core::mem::transmute(v))) }
}

pub fn phys_to_kernel_mut<T>(v: PhysMutRef<T>) -> &mut T {
    unsafe { core::mem::transmute(phys_to_kernel_usize(core::mem::transmute(v))) }
}

pub fn kernel_to_phys<T>(v: &T) -> PhysRef<T> {
    unsafe { core::mem::transmute(kernel_to_phys_usize(core::mem::transmute(v))) }
}

pub fn kernel_to_phys_mut<T>(v: &mut T) -> PhysMutRef<T> {
    unsafe { core::mem::transmute(kernel_to_phys_usize(core::mem::transmute(v))) }
}
