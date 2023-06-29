pub mod asm_utils;
pub mod cpu;
pub mod trap;

extern crate r0;
extern crate rlibc;

// pub mod clint;
// pub mod critical;
// pub mod plic;
// pub mod sbi;
// pub mod timer;
// pub mod trap;
// pub mod wrapper;

extern "C" {
    static mut _ebss: u64;
    static mut _sbss: u64;

    static mut _edata: u64;
    static mut _sdata: u64;

    static mut _sidata: u64;
}

/// # Safety
/// Function has to initialize stack and data regions
/// Has to zero bss and init data
/// Assumes that correct device tree header/struct and hartid is passed
#[no_mangle]
pub unsafe extern "C" fn _start_rust(argc: u32, argv: *const *const core::ffi::c_char) -> ! {
    extern "C" {
        fn kernel_main_elf(argc: u32, argv: *const *const core::ffi::c_char);
    }

    extern "Rust" {
        fn _setup_interrupts();
    }

    //kernel_main(hartid, 0, dtb);
    kernel_main_elf(argc, argv);

    shutdown();
}

pub fn shutdown() -> ! {
    extern "C" {
        fn wfi_spin() -> !;
    }
    unsafe { wfi_spin() }
}
