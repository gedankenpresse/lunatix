use elfloader::ElfBinary;
use librust::{
    prelude::*,
    syscall_abi::{identify::CapabilityVariant, map_page::MapPageFlag},
};

use crate::{
    commands::Command, elfloader::LunatixElfLoader, CADDR_CHILD_CSPACE, CADDR_CHILD_PAGE_START,
    CADDR_CHILD_STACK_PAGE, CADDR_CHILD_TASK, CADDR_CHILD_VSPACE, CADDR_MEM, CADDR_VSPACE,
    HELLO_WORLD_BIN,
};

pub struct SecondTask;

impl Command for SecondTask {
    fn get_name(&self) -> &'static str {
        "second_task"
    }

    fn get_summary(&self) -> &'static str {
        "load another elf binary from init"
    }

    fn execute(&self, args: &str) -> Result<(), ()> {
        run_second_task();
        Ok(())
    }
}

fn run_second_task() {
    librust::derive_from_mem(CADDR_MEM, CADDR_CHILD_TASK, CapabilityVariant::Task, None).unwrap();

    librust::derive_from_mem(
        CADDR_MEM,
        CADDR_CHILD_CSPACE,
        CapabilityVariant::CSpace,
        Some(8),
    )
    .unwrap();
    librust::task_assign_cspace(CADDR_CHILD_CSPACE, CADDR_CHILD_TASK).unwrap();

    librust::derive_from_mem(
        CADDR_MEM,
        CADDR_CHILD_VSPACE,
        CapabilityVariant::VSpace,
        None,
    )
    .unwrap();
    assert_eq!(
        librust::identify(CADDR_CHILD_VSPACE).unwrap(),
        CapabilityVariant::VSpace
    );
    librust::task_assign_vspace(CADDR_CHILD_VSPACE, CADDR_CHILD_TASK).unwrap();

    println!("loading HelloWorld binary");
    // load a stack for the child task
    const CHILD_STACK_LOW: usize = 0x5_0000_0000;
    librust::derive_from_mem(
        CADDR_MEM,
        CADDR_CHILD_STACK_PAGE,
        CapabilityVariant::Page,
        None,
    )
    .unwrap();
    librust::map_page(
        CADDR_CHILD_STACK_PAGE,
        CADDR_CHILD_VSPACE,
        CADDR_MEM,
        CHILD_STACK_LOW,
        MapPageFlag::READ | MapPageFlag::WRITE,
    )
    .unwrap();
    // load binary elf code
    let elf_binary = ElfBinary::new(HELLO_WORLD_BIN).unwrap();
    let mut elf_loader = LunatixElfLoader::<8>::new(
        CADDR_MEM,
        CADDR_VSPACE,
        CADDR_CHILD_VSPACE,
        CADDR_CHILD_PAGE_START,
        0x0000003000000000,
    );
    elf_binary.load(&mut elf_loader).unwrap();
    librust::task_assign_control_registers(
        CADDR_CHILD_TASK,
        elf_binary.entry_point() as usize,
        CHILD_STACK_LOW + 4096,
        0x0,
        0x0,
    )
    .unwrap();
    println!("Yielding to Hello World Task");
    librust::yield_to(CADDR_CHILD_TASK).unwrap();
}
