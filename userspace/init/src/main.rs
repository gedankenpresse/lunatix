#![no_std]
#![no_main]

use core::panic::PanicInfo;
use librust::syscall_abi::alloc_page::AllocPageReturn;
use librust::syscall_abi::identify::{CapabilityVariant, IdentifyReturn};
use librust::syscall_abi::CAddr;
use librust::{alloc_page, println};

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

    const CADDR_ALLOCATED_PAGE: CAddr = 4;
    assert_eq!(
        alloc_page(CADDR_MEM, CADDR_ALLOCATED_PAGE),
        AllocPageReturn::Success
    );

    println!("👋 Init task says good bye");
}

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    println!("panic {}", info);
    loop {}
}
