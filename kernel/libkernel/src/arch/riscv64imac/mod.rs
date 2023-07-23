pub use riscv::abort;
pub use riscv::shutdown;

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
pub unsafe extern "C" fn _start_rust(
    argc: u32,
    argv: *const *const core::ffi::c_char,
    phys_fdt: *const u8,
    phys_mem_start: *mut u8,
    phys_mem_end: *mut u8,
) -> ! {
    extern "C" {
        fn kernel_main_elf(
            argc: u32,
            argv: *const *const core::ffi::c_char,
            phys_fdt: *const u8,
            phys_mem_start: *mut u8,
            phys_mem_end: *mut u8,
        );
    }

    extern "Rust" {
        fn _setup_interrupts();
    }

    //kernel_main(hartid, 0, dtb);
    kernel_main_elf(argc, argv, phys_fdt, phys_mem_start, phys_mem_end);

    riscv::shutdown();
}
