#![no_std]
#![no_main]

use librust::println;

#[no_mangle]
fn _start() {
    main();
}

static MESSAGE: &'static str = ":This is a very long userspace message from outer space!";

fn main() {
    println!("hello word!");
    println!("{}", MESSAGE);
    println!("{}", MESSAGE);
}

use core::panic::PanicInfo;

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    println!("panic {}", info);
    loop {}
}
