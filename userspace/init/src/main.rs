#![no_std]
#![no_main]

use core::panic::PanicInfo;
use librust::println;
use librust::syscall_abi::identify::{CapabilityVariant, IdentifyReturn};

#[no_mangle]
fn _start() {
    main();
}

static MESSAGE: &'static str = ":This is a very long userspace message from outer space!";

const CADDR_MEM: usize = 1;
const CADDR_CSPACE: usize = 2;
const CADDR_VSPACE: usize = 3;

fn main() {
    println!("{}", MESSAGE);
    println!("{}", MESSAGE);

    assert_eq!(
        librust::identify(CADDR_MEM),
        IdentifyReturn::Success(CapabilityVariant::Memory)
    );
    assert_eq!(
        librust::identify(CADDR_CSPACE),
        IdentifyReturn::Success(CapabilityVariant::CSpace)
    );
    assert_eq!(
        librust::identify(CADDR_VSPACE),
        IdentifyReturn::Success(CapabilityVariant::VSpace)
    );

    println!("👋 Init task says good bye");
}

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    println!("panic {}", info);
    loop {}
}
