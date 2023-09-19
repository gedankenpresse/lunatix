#![no_std]
#![no_main]

use core::panic::PanicInfo;
use librust::println;
use librust::syscall_abi::identify::{CapabilityVariant};
use librust::syscall_abi::CAddr;
use librust::syscall_abi::derive_from_mem::DeriveFromMemReturn;
use librust::syscall_abi::task_assign_cspace::TaskAssignCSpaceReturn;
use librust::syscall_abi::task_assign_vspace::TaskAssignVSpaceReturn;

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

    const CADDR_NEW_TASK: CAddr = 4;
    assert_eq!(
        librust::derive_from_mem(CADDR_MEM, CADDR_NEW_TASK, CapabilityVariant::Task, None),
        DeriveFromMemReturn::Success,
    );

    const CADDR_NEW_CSPACE: CAddr = 5;
    assert_eq!(
        librust::derive_from_mem(CADDR_MEM, CADDR_NEW_CSPACE, CapabilityVariant::CSpace, Some(8)),
        DeriveFromMemReturn::Success,
    );
    assert_eq!(
        librust::task_assign_cspace(CADDR_NEW_CSPACE, CADDR_NEW_TASK),
        TaskAssignCSpaceReturn::Success,
    );

    const CADDR_NEW_VSPACE: CAddr = 6;
    assert_eq!(
        librust::derive_from_mem(CADDR_MEM, CADDR_NEW_VSPACE, CapabilityVariant::VSpace, None),
        DeriveFromMemReturn::Success,
    );
    assert_eq!(
        librust::task_assign_vspace(CADDR_NEW_VSPACE, CADDR_NEW_TASK),
        TaskAssignVSpaceReturn::Success,
    );

    println!("Init task says good bye 👋");
}

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    println!("panic {}", info);
    loop {}
}
