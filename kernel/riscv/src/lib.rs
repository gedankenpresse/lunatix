#![no_std]

mod asm_utils;
pub mod cpu;
pub mod pt;
pub mod timer;
pub mod trap;

pub use asm_utils::{wait_for_interrupt, wfi_spin};

#[cfg(target_arch = "riscv64")]
#[inline(always)]
pub fn shutdown() -> ! {
    use log::error;
    use sbi::system_reset::*;
    match system_reset(ResetType::Shutdown, ResetReason::NoReason) {
        Ok(_) => {}
        Err(e) => error!("shutdown error: {}", e),
    };
    sbi::legacy::shutdown();
    #[allow(unreachable_code)]
    unsafe {
        crate::wfi_spin()
    }
}

#[cfg(target_arch = "riscv64")]
#[inline(always)]
pub fn abort() -> ! {
    use log::error;
    use sbi::system_reset::*;
    match system_reset(ResetType::Shutdown, ResetReason::SystemFailure) {
        Ok(_) => {}
        Err(e) => error!("abort error: {}", e),
    };
    sbi::legacy::shutdown();
    #[allow(unreachable_code)]
    unsafe {
        crate::wfi_spin()
    }
}

pub unsafe trait PhysMapper {
    unsafe fn phys_to_mapped_mut<T>(&self, phys: *mut T) -> *mut T;
    unsafe fn phys_to_mapped<T>(&self, phys: *const T) -> *const T;
    unsafe fn mapped_to_phys_mut<T>(&self, mapped: *mut T) -> *mut T;
    unsafe fn mapped_to_phys<T>(&self, mapped: *const T) -> *const T;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
