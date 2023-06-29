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
        fn __pre_init();
        fn _mp_hook() -> bool;
        fn _setup_interrupts();
    }

    if _mp_hook() {
        __pre_init();

        // this seems to be done by uboot, we don't have to do this manually
        //r0::zero_bss(&mut _sbss, &mut _ebss);
        //r0::init_data(&mut _sdata, &mut _edata, &_sidata);
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

#[no_mangle]
pub fn default_pre_init() {}

/// # Safety
/// should only return true for one hart, but with sbi it shouldn't matter
#[no_mangle]
pub unsafe fn default_mp_hook() -> bool {
    //when booting with sbi only one hart should be executing, so we can do nothing here
    true
}
