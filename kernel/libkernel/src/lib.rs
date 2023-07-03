#![no_std]

#[path = "arch/riscv64imac/mod.rs"]
pub mod arch;

pub mod argv_iter;

#[macro_export]
#[macro_use]
mod print;
