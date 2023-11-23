#![no_std]
#![no_main]

use core::panic::PanicInfo;
use liblunatix::prelude::*;

#[no_mangle]
fn _start() {
    main();
    liblunatix::syscalls::exit();
}

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    println!("echo_client panicked {}", info);
    liblunatix::syscalls::exit();
}

fn main() {
    println!("echo_client started");
    const ENDPOINT_CADDR: CAddr = CAddr::new(1, 1);
    assert_eq!(
        liblunatix::syscalls::identify(ENDPOINT_CADDR),
        Ok(CapabilityVariant::Endpoint)
    );

    for i in 0..10_000 {
        liblunatix::syscalls::send(ENDPOINT_CADDR, 0, &[], &[0x55, i]).unwrap();
    }
}
