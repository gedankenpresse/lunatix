#![no_std]
// TODO: remove dead code
#![allow(dead_code)]
#![allow(unused_variables)]

pub mod caps;
pub mod init;
pub mod ipc;
pub mod sched;
pub mod uapi;
pub mod virtmem;

#[cfg(target_arch = "riscv64")]
#[path = "arch/riscv64imac/mod.rs"]
mod arch_specific;

#[cfg(target_arch = "x86_64")]
#[path = "arch/x86_64/mod.rs"]
mod arch_specific;

pub use arch_specific::mmu;
pub use arch_specific::trap;
use caps::CSlot;
use ksync::SpinLock;
use libkernel::mem::ptrs::PhysConstPtr;
use riscv::pt::PageTable;

pub struct InitCaps {
    mem: CSlot,
    init_task: CSlot,
}

impl InitCaps {
    const fn empty() -> Self {
        Self {
            mem: CSlot::empty(),
            init_task: CSlot::empty(),
        }
    }
}

/// TODO: fix this somehow
/// CSlot isn't send because raw pointers... meh
unsafe impl Send for InitCaps {}

pub static INIT_CAPS: SpinLock<InitCaps> = SpinLock::new(InitCaps::empty());

pub static mut KERNEL_ROOT_PT: PhysConstPtr<PageTable> = PhysConstPtr::null();
