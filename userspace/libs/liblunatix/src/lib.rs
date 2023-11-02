#![no_std]

#[macro_use]
pub mod syscalls;

pub mod ipc;

pub mod prelude {
    pub use crate::print;
    pub use crate::println;
    pub use syscall_abi;
    pub use syscall_abi::CAddr;
    pub use syscall_abi::NoValue;
    pub use syscall_abi::SyscallError;
}
