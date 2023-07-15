#![no_std]
#![no_main]

use librust::println;

#[no_mangle]
fn _start() {
    main();
}

static MESSAGE: &'static str = ":This is a very long userspace message from outer space!";

const MEM_CAP: usize = 1;
const CSPACE_CAP: usize = 2;
const NEW_TASK_CAP: usize = 3;
const NEW_VSPACE_CAP: usize = 4;
const NEW_CSPACE_CAP: usize = 5;

fn main() {
    println!("hello word!");
    println!("{}", MESSAGE);
    println!("{}", MESSAGE);

    assert_eq!(librust::identify(MEM_CAP), Ok(librust::Variant::Memory));
    assert_eq!(librust::identify(CSPACE_CAP), Ok(librust::Variant::CSpace));
    librust::allocate(MEM_CAP, NEW_TASK_CAP, librust::Variant::Task, 0).unwrap();
    println!("new alloc: {:?}", librust::identify(NEW_TASK_CAP));

    librust::allocate(MEM_CAP, NEW_CSPACE_CAP, librust::Variant::CSpace, 4).unwrap();
    println!("new alloc: {:?}", librust::identify(NEW_CSPACE_CAP));



    librust::allocate(MEM_CAP, NEW_VSPACE_CAP, librust::Variant::VSpace, 0).unwrap();
    println!("new alloc: {:?}", librust::identify(NEW_VSPACE_CAP));

}

use core::panic::PanicInfo;

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    println!("panic {}", info);
    loop {}
}
