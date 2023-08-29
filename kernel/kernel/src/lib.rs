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
pub use arch_specific::mmu;
pub use arch_specific::trap;
use caps::Capability;
use ksync::SpinLock;
use libkernel::mem::ptrs::PhysConstPtr;
use riscv::pt::PageTable;

pub struct InitCaps {
    pub mem: Capability,
    pub init_task: Capability,
}

impl InitCaps {
    /// Create a new instance with uninitialized capabilities
    const fn empty() -> Self {
        Self {
            mem: Capability::empty(),
            init_task: Capability::empty(),
        }
    }
}

/// TODO: fix this somehow
/// CSlot isn't send because raw pointers... meh
unsafe impl Send for InitCaps {}

/// A global static holding the capabilities given to the init task
pub static INIT_CAPS: SpinLock<InitCaps> = SpinLock::new(InitCaps::empty());

/// A global static reference to the root PageTable which has only the kernel part mapped
pub static mut KERNEL_ROOT_PT: PhysConstPtr<PageTable> = PhysConstPtr::null();

pub static mut KERNEL_ALLOCATOR: Option<KernelAlloc> = None;
