use alloc::vec;
use alloc::vec::Vec;
use elfloader::ElfBinary;
use librust::prelude::CAddr;
use librust::println;
use librust::syscall_abi::identify::CapabilityVariant;
use librust::syscall_abi::yield_to::TaskStatus;
use librust::syscall_abi::MapFlags;

use crate::caddr_alloc::alloc_caddr;
use crate::elfloader::LunatixElfLoader;
use crate::sched::Scheduler;
use crate::{read::Reader, CADDR_ASID_CONTROL, CADDR_MEM, CADDR_VSPACE, FS};

use super::Command;

pub struct Exec;

struct TaskCaps {
    task: CAddr,
    cspace: CAddr,
    vspace: CAddr,
    stack_page: CAddr,
}

impl Command for Exec {
    fn get_name(&self) -> &'static str {
        "exec"
    }

    fn get_summary(&self) -> &'static str {
        "execute a binary (currently only loads a binary)"
    }

    fn execute(&self, args: &str) -> Result<(), &'static str> {
        let mut tasks = Vec::new();
        for path in args.split(" ") {
            log::debug!("reading binary {path:?} from filesystem");
            let mut p9 = FS.0.borrow_mut();
            let p9 = p9.as_mut().unwrap();
            let mut reader = p9.read_file(&[path]).unwrap();
            let file_bin = reader.read_to_vec(16).unwrap();

            // load the elf content into the task
            log::debug!("preparing capabilities for the new task");
            let task_caps = self.make_task_caps();
            librust::asid_assign(CADDR_ASID_CONTROL, task_caps.vspace).unwrap();

            // load a stack for the child task
            log::debug!("mapping stack space for the new task");
            const TASK_STACK_LOW: usize = 0x5_0000_0000;
            librust::map_page(
                task_caps.stack_page,
                task_caps.vspace,
                CADDR_MEM,
                TASK_STACK_LOW,
                MapFlags::READ | MapFlags::WRITE,
            )
            .unwrap();

            // load the elf content
            log::debug!("loading {} elf code", path);
            let elf_binary = ElfBinary::new(&file_bin).unwrap();
            let mut elf_loader =
                LunatixElfLoader::new(CADDR_MEM, CADDR_VSPACE, task_caps.vspace, 0x31_0000_0000);
            elf_binary.load(&mut elf_loader).unwrap();
            log::debug!("3");
            elf_loader.remap_to_target_vspace();

            // setting task start params
            librust::task_assign_control_registers(
                task_caps.task,
                elf_binary.entry_point() as usize,
                TASK_STACK_LOW + 4096,
                0x0,
                0x0,
            )
            .unwrap();

            tasks.push(task_caps);
        }

        // run the tasks
        let mut sched = Scheduler::new(tasks.iter().map(|caps| caps.task));
        sched.run_schedule();

        // TODO Cleanup the task objects

        Ok(())
    }
}

impl Exec {
    fn make_task_caps(&self) -> TaskCaps {
        let task = alloc_caddr();
        librust::derive(CADDR_MEM, task, CapabilityVariant::Task, None).unwrap();

        let cspace = alloc_caddr();
        librust::derive(CADDR_MEM, cspace, CapabilityVariant::CSpace, Some(8)).unwrap();
        librust::task_assign_cspace(cspace, task).unwrap();

        let vspace = alloc_caddr();
        librust::derive(CADDR_MEM, vspace, CapabilityVariant::VSpace, None).unwrap();
        librust::task_assign_vspace(vspace, task).unwrap();

        let stack_page = alloc_caddr();
        librust::derive(CADDR_MEM, stack_page, CapabilityVariant::Page, None).unwrap();

        TaskCaps {
            task,
            cspace,
            vspace,
            stack_page,
        }
    }

    fn destroy_task_caps(&self, caps: TaskCaps) {
        librust::destroy(caps.stack_page).unwrap();
        librust::destroy(caps.cspace).unwrap();
        librust::destroy(caps.task).unwrap();
        librust::destroy(caps.vspace).unwrap();
    }
}
