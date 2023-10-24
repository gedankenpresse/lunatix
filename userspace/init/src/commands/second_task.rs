use alloc::vec::Vec;
use elfloader::ElfBinary;
use librust::{
    prelude::*,
    syscall_abi::{identify::CapabilityVariant, MapFlags},
};

use crate::read::Reader;
use crate::{
    caddr_alloc, commands::Command, elfloader::LunatixElfLoader, CADDR_ASID_CONTROL, CADDR_MEM,
    CADDR_VSPACE, FS,
};

pub struct SecondTask;

impl Command for SecondTask {
    fn get_name(&self) -> &'static str {
        "second_task"
    }

    fn get_summary(&self) -> &'static str {
        "load another elf binary from init"
    }

    fn execute(&self, _args: &str) -> Result<(), &'static str> {
        run_second_task();
        Ok(())
    }
}

fn run_second_task() {
    let task = caddr_alloc::alloc_caddr();
    librust::derive(CADDR_MEM, task, CapabilityVariant::Task, None).unwrap();

    let cspace = caddr_alloc::alloc_caddr();
    librust::derive(CADDR_MEM, cspace, CapabilityVariant::CSpace, Some(8)).unwrap();
    librust::task_assign_cspace(cspace, task).unwrap();

    let vspace = caddr_alloc::alloc_caddr();
    librust::derive(CADDR_MEM, vspace, CapabilityVariant::VSpace, None).unwrap();
    assert_eq!(
        librust::identify(vspace).unwrap(),
        CapabilityVariant::VSpace
    );
    librust::asid_assign(CADDR_ASID_CONTROL, vspace).unwrap();
    librust::task_assign_vspace(vspace, task).unwrap();

    // load a stack for the child task
    let stack_page = caddr_alloc::alloc_caddr();
    const CHILD_STACK_LOW: usize = 0x5_0000_0000;
    librust::derive(CADDR_MEM, stack_page, CapabilityVariant::Page, None).unwrap();
    librust::map_page(
        stack_page,
        vspace,
        CADDR_MEM,
        CHILD_STACK_LOW,
        MapFlags::READ | MapFlags::WRITE,
    )
    .unwrap();

    log::info!("loading HelloWorld binary from filesystem");
    let mut fs = FS.0.borrow_mut();
    let mut fs = fs.as_mut().unwrap();
    let mut file_reader = fs.read_file(&["hello_world"]).unwrap();
    let file_bin = file_reader.read_to_vec().unwrap();

    log::info!("load binary elf code");
    // load binary elf code
    let elf_binary = ElfBinary::new(&file_bin).unwrap();
    log::info!("calling elf loader new");
    let mut elf_loader =
        LunatixElfLoader::<4>::new(CADDR_MEM, CADDR_VSPACE, vspace, 0x31_0000_0000);
    log::info!("calling elf loader");
    elf_binary.load(&mut elf_loader).unwrap();
    librust::task_assign_control_registers(
        task,
        elf_binary.entry_point() as usize,
        CHILD_STACK_LOW + 4096,
        0x0,
        0x0,
    )
    .unwrap();
    elf_loader.remap_to_target_vspace();
    println!("Yielding to Hello World Task");
    librust::yield_to(task).unwrap();
}
