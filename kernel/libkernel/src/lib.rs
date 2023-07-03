#![no_std]

#[path = "arch/riscv64imac/mod.rs"]
pub mod arch;
#[macro_export]
#[macro_use]
mod print;
