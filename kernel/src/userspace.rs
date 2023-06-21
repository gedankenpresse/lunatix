use crate::println;

/// A function that behaves as a userspace program would for testing code
#[no_mangle]
pub fn fake_userspace() -> ! {
    println!("Hello from Userspace");
    unsafe {
        core::arch::asm!(
            "ecall",
            in("a0") 1,
        );
    }
    println!("Hello after syscall");
    loop {
        unsafe {
            let null_deref = *(0 as *mut u8);
            println!("{null_deref}");
        };
    }
}
