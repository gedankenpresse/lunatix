#![no_std]

#[cfg(target_arch = "riscv64")]
#[path = "arch/riscv64imac/mod.rs"]
pub mod arch;

#[cfg(target_arch = "x86_64")]
#[path = "arch/x86_64/mod.rs"]
pub mod arch;

pub mod argv_iter;

#[macro_use]
pub mod print;
pub mod device_info;
pub mod log;
pub mod mem;
