//! Minimal trap handling for the kernel loader

use core::arch::asm;
use riscv::cpu::StVecData;
use riscv::trap::TrapInfo;

/// Configure the kernel_loader trap handler to be used by the CPU
pub fn set_trap_handler() {
    let handler = handle_trap as u64;
    log::debug!("configuring CPU for kernel_loader trap handler at {handler:0x}");
    unsafe {
        riscv::cpu::StVec::write(&StVecData {
            mode: 0,
            base: handler,
        })
    }
}

/// The actual trap handler for the kernel_loader
fn handle_trap() {
    unsafe {
        asm!(".align 8");
    }
    let trap_info = TrapInfo::from_current_regs();
    panic!("Caught System Trap: {:#x?}", trap_info)
}
