#![no_std]

#[macro_use]
pub mod syscalls;
pub mod ipc;
pub mod print;

pub mod prelude {
    pub use crate::print;
    pub use crate::println;
    pub use print::SYS_WRITER;
    pub use syscall_abi;
    pub use syscall_abi::CAddr;
    pub use syscall_abi::NoValue;
    pub use syscall_abi::SyscallError;
}

#[repr(C, align(4096))]
pub struct MemoryPage {
    bytes: [u8; 4096],
}
