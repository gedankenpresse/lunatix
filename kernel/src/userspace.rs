use crate::println;

/// A function that behaves as a userspace program would for testing code
#[no_mangle]
pub fn fake_userspace() -> ! {
    println!("Hello from Userspace");
    loop {
        unsafe {
            let null_deref = *(0 as *mut u8);
            println!("{null_deref}");
        };
    }
}
