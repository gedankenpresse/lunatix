#![no_std]
// TODO: remove dead code
#![allow(dead_code)]
#![allow(unused_variables)]

pub mod caps;
pub mod init;
pub mod ipc;
pub mod sched;
pub mod virtmem;

#[cfg(target_arch = "riscv64")]
#[path = "arch/riscv64imac/mod.rs"]
mod arch_specific;

#[cfg(target_arch = "x86_64")]
#[path = "arch/x86_64/mod.rs"]
mod arch_specific;
mod syscalls;

use crate::caps::KernelAlloc;
use allocators::Box;
pub use arch_specific::mmu;
pub use arch_specific::trap;
use caps::Capability;
use libkernel::mem::ptrs::PhysConstPtr;
use riscv::pt::PageTable;

pub struct InitCaps<'alloc, 'mem> {
    pub init_task: Box<'alloc, 'mem, Capability>,
}

/// A global static reference to the root PageTable which has only the kernel part mapped
pub static mut KERNEL_ROOT_PT: PhysConstPtr<PageTable> = PhysConstPtr::null();

pub static mut KERNEL_ALLOCATOR: Option<KernelAlloc> = None;
