#![no_std]
#![no_main]

use core::panic::PanicInfo;
use liblunatix::println;

#[no_mangle]
fn _start() {
    main();
    liblunatix::exit();
}

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    println!("hello_world panicked {}", info);
    liblunatix::exit();
}

fn main() {
    for i in 0..3 {
        println!("Hello World {i:}");
        liblunatix::r#yield().unwrap();
    }
}
