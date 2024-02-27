#![no_std]

// data structures that are riscv specific but implemented in pure rust
pub mod mem;
pub mod pt;

// actual riscv specific parts
#[cfg(target_arch = "riscv64")]
pub mod cpu;
#[cfg(target_arch = "riscv64")]
pub mod power;
#[cfg(target_arch = "riscv64")]
pub mod timer;
#[cfg(target_arch = "riscv64")]
pub mod trap;
#[cfg(target_arch = "riscv64")]
pub mod utils;

#[deprecated]
pub unsafe trait PhysMapper {
    unsafe fn phys_to_mapped_mut<T>(&self, phys: *mut T) -> *mut T;
    unsafe fn phys_to_mapped<T>(&self, phys: *const T) -> *const T;
    unsafe fn mapped_to_phys_mut<T>(&self, mapped: *mut T) -> *mut T;
    unsafe fn mapped_to_phys<T>(&self, mapped: *const T) -> *const T;
}
