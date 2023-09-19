#![no_std]
#![no_main]

use core::panic::PanicInfo;
use librust::println;
use librust::syscall_abi::identify::{CapabilityVariant, IdentifyReturn};
use librust::syscall_abi::CAddr;
use librust::syscall_abi::derive_from_mem::DeriveFromMemReturn;

#[no_mangle]
fn _start() {
    main();
}

static MESSAGE: &'static str = ":This is a very long userspace message from outer space!";

const CADDR_MEM: CAddr = 1;
const CADDR_CSPACE: CAddr = 2;
const CADDR_VSPACE: CAddr = 3;

fn main() {
    println!("{}", MESSAGE);
    println!("{}", MESSAGE);

    const CADDR_ALLOCATED_PAGE: CAddr = 4;
    assert_eq!(
        librust::derive_from_mem(CADDR_MEM, CADDR_ALLOCATED_PAGE, CapabilityVariant::Page, None),
        DeriveFromMemReturn::Success,
    );

    println!("Init task says good bye 👋");
}

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    println!("panic {}", info);
    loop {}
}
