#![no_std]

#[macro_use]
pub mod print;
mod kernel_logger;

pub use kernel_logger::KernelLogger;
pub use print::KernelWriter;
