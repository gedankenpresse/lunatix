#![no_std]
#![no_main]

use core::panic::PanicInfo;
use librust::println;

#[no_mangle]
fn _start() {
    main();
    librust::exit();
}

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    println!("hello_world panicked {}", info);
    librust::exit();
}

fn main() {
    for i in 0..3 {
        println!("Hello World {i:}");
        librust::r#yield().unwrap();
    }
}
