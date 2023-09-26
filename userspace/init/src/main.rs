#![no_std]
#![no_main]

mod elfloader;

use crate::elfloader::LunatixElfLoader;
use ::elfloader::ElfBinary;
use core::panic::PanicInfo;
use librust::println;
use librust::syscall_abi::derive_from_mem::DeriveFromMemReturn;
use librust::syscall_abi::identify::{CapabilityVariant, IdentifyReturn};
use librust::syscall_abi::map_page::{MapPageFlag, MapPageReturn};
use librust::syscall_abi::task_assign_control_registers::TaskAssignControlRegistersReturn;
use librust::syscall_abi::task_assign_cspace::TaskAssignCSpaceReturn;
use librust::syscall_abi::task_assign_vspace::TaskAssignVSpaceReturn;
use librust::syscall_abi::yield_to::YieldToReturn;
use librust::syscall_abi::CAddr;

static HELLO_WORLD_BIN: &[u8] =
    include_bytes!("../../../target/riscv64imac-unknown-none-elf/release/hello_world");

#[no_mangle]
fn _start() {
    main();
}

const CADDR_MEM: CAddr = 1;
const CADDR_CSPACE: CAddr = 2;
const CADDR_VSPACE: CAddr = 3;

fn main() {
    const CADDR_CHILD_TASK: CAddr = 4;
    assert_eq!(
        librust::derive_from_mem(CADDR_MEM, CADDR_CHILD_TASK, CapabilityVariant::Task, None),
        DeriveFromMemReturn::Success,
    );

    const CADDR_CHILD_CSPACE: CAddr = 5;
    assert_eq!(
        librust::derive_from_mem(
            CADDR_MEM,
            CADDR_CHILD_CSPACE,
            CapabilityVariant::CSpace,
            Some(8),
        ),
        DeriveFromMemReturn::Success,
    );
    assert_eq!(
        librust::task_assign_cspace(CADDR_CHILD_CSPACE, CADDR_CHILD_TASK),
        TaskAssignCSpaceReturn::Success,
    );

    const CADDR_CHILD_VSPACE: CAddr = 6;
    assert_eq!(
        librust::derive_from_mem(
            CADDR_MEM,
            CADDR_CHILD_VSPACE,
            CapabilityVariant::VSpace,
            None
        ),
        DeriveFromMemReturn::Success,
    );
    assert_eq!(
        librust::identify(CADDR_CHILD_VSPACE),
        IdentifyReturn::Success(CapabilityVariant::VSpace)
    );
    assert_eq!(
        librust::task_assign_vspace(CADDR_CHILD_VSPACE, CADDR_CHILD_TASK),
        TaskAssignVSpaceReturn::Success,
    );

    println!("loading HelloWorld binary");
    // load a stack for the child task
    const CADDR_CHILD_STACK_PAGE: CAddr = 7;
    const CHILD_STACK_LOW: usize = 0x5_0000_0000;
    assert_eq!(
        librust::derive_from_mem(
            CADDR_MEM,
            CADDR_CHILD_STACK_PAGE,
            CapabilityVariant::Page,
            None
        ),
        DeriveFromMemReturn::Success,
    );
    assert_eq!(
        librust::map_page(
            CADDR_CHILD_STACK_PAGE,
            CADDR_CHILD_VSPACE,
            CADDR_MEM,
            CHILD_STACK_LOW,
            MapPageFlag::READ | MapPageFlag::WRITE
        ),
        MapPageReturn::Success
    );

    // load binary elf code
    const CADDR_CHILD_PAGE_START: CAddr = 8;
    let elf_binary = ElfBinary::new(HELLO_WORLD_BIN).unwrap();
    let mut elf_loader = LunatixElfLoader::<8>::new(
        CADDR_MEM,
        CADDR_VSPACE,
        CADDR_CHILD_VSPACE,
        CADDR_CHILD_PAGE_START,
        0x0000003000000000,
    );
    elf_binary.load(&mut elf_loader).unwrap();
    assert_eq!(
        librust::task_assign_control_registers(
            CADDR_CHILD_TASK,
            elf_binary.entry_point() as usize,
            CHILD_STACK_LOW + 4096,
            0x0,
            0x0
        ),
        TaskAssignControlRegistersReturn::Success
    );

    println!("Yielding to Hello World Task");
    assert_eq!(librust::yield_to(CADDR_CHILD_TASK), YieldToReturn::Success);

    println!("Init task says good bye 👋");
}

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    println!("panic {}", info);
    loop {}
}
