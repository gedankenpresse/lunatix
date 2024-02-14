#![no_std]

mod asm_utils;
pub mod cpu;
pub mod mem;
pub mod power;
pub mod pt;
pub mod timer;
pub mod trap;

pub use asm_utils::{wait_for_interrupt, wfi_spin};

pub unsafe trait PhysMapper {
    unsafe fn phys_to_mapped_mut<T>(&self, phys: *mut T) -> *mut T;
    unsafe fn phys_to_mapped<T>(&self, phys: *const T) -> *const T;
    unsafe fn mapped_to_phys_mut<T>(&self, mapped: *mut T) -> *mut T;
    unsafe fn mapped_to_phys<T>(&self, mapped: *const T) -> *const T;
}
