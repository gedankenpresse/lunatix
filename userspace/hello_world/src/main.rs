#![no_std]
#![no_main]

use core::panic::PanicInfo;
use librust::println;
use librust::syscall_abi::r#yield::YieldReturn;

#[no_mangle]
fn _start() {
    main();
}

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    println!("hello_world panicked {}", info);
    loop {}
}

fn main() {
    println!("Hello World");
    loop {
        assert_eq!(librust::r#yield(), YieldReturn::Success);
    }
}
