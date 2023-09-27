#![no_std]
#![no_main]

use core::panic::PanicInfo;
use librust::println;

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
        librust::r#yield().unwrap();
    }
}
