#![no_std]
#![no_main]

mod logger;

use crate::logger::Logger;
use core::ops::{Range, RangeBounds};
use core::panic::PanicInfo;
use liblunatix::prelude::CAddr;
use liblunatix::println;
use log::Level;

static LOGGER: Logger = Logger::new(Level::Info);

#[no_mangle]
fn _start() {
    LOGGER.install().expect("could not install logger");
    main();
    liblunatix::syscalls::exit();
}

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    println!("hello_world panicked {}", info);
    liblunatix::syscalls::exit();
}

fn main() {
    inspect_caddrs(0..10);
}

fn inspect_caddrs(range: Range<usize>) {
    for raw_addr in range {
        let addr = CAddr::from(raw_addr);
        match liblunatix::syscalls::identify(addr) {
            Err(e) => log::error!("Could not inspect {addr:?}: {e:?}"),
            Ok(identity) => log::info!("{addr:?} = {identity:?}"),
        };
    }
}
