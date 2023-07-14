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

    assert_eq!(librust::identify(1), Ok(librust::Variant::Memory));
    librust::allocate(1, 2, librust::Variant::Task, 0).unwrap();
    println!("new alloc: {:?}", librust::identify(2));

    librust::allocate(1, 3, librust::Variant::CSpace, 4).unwrap();
    println!("new alloc: {:?}", librust::identify(3));



    librust::allocate(1, 4, librust::Variant::VSpace, 0).unwrap();
    println!("new alloc: {:?}", librust::identify(4));

}

use core::panic::PanicInfo;

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    println!("panic {}", info);
    loop {}
}
