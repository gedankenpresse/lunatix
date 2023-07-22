#![no_std]
#![cfg_attr(test, no_main)]
// TODO: remove dead code
#![allow(dead_code)]
#![allow(unused_variables)]
#![cfg_attr(test, feature(custom_test_frameworks))]
#![cfg_attr(test, test_runner(test_runner))]

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

#[cfg(test)]
use core::panic::PanicInfo;

pub use arch_specific::mmu;
pub use arch_specific::trap;
use caps::CSlot;
use ksync::SpinLock;
#[cfg(test)]
use libkernel::log::KernelLogger;
use libkernel::mem::ptrs::PhysConstPtr;
#[cfg(test)]
use libkernel::mem::ptrs::PhysMutPtr;
use libkernel::mem::PageTable;

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

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    test_panic_handler(info)
}

#[cfg(test)]
pub fn test_panic_handler(info: &PanicInfo) -> ! {
    use libkernel::println;

    println!("[failed]\n");
    println!("Error: {}\n", info);
    libkernel::arch::abort();
}

#[cfg(test)]
fn test_runner(tests: &[&dyn Fn()]) {
    libkernel::println!("Running {} tests", tests.len());
    for test in tests {
        test();
    }
}
