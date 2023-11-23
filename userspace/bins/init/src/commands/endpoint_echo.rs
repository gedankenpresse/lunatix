use crate::commands::Command;
use crate::elfloader::LunatixElfLoader;
use crate::sched::Scheduler;
use crate::{CADDR_ASID_CONTROL, CADDR_MEM, CADDR_VSPACE, CSPACE_BITS, FS};
use caddr_alloc::alloc_caddr;
use elfloader::ElfBinary;
use io::read::Reader;
use liblunatix::prelude::syscall_abi::identify::CapabilityVariant;
use liblunatix::prelude::syscall_abi::MapFlags;
use liblunatix::prelude::CAddr;

pub struct EndpointEcho;

struct TaskCaps {
    task: CAddr,
    cspace: CAddr,
    vspace: CAddr,
    stack_page: CAddr,
}

impl EndpointEcho {
    fn load_binary(&self, path: &str) -> TaskCaps {
        let task_caps = self.make_task_caps();

        log::debug!("reading binary {path:?} from filesystem");
        let mut p9 = FS.0.borrow_mut();
        let p9 = p9.as_mut().unwrap();
        let mut reader = p9.read_file(&[path]).unwrap();
        let file_bin = reader.read_to_vec(16).unwrap();

        // load the elf content into the task
        log::debug!("preparing capabilities for the new task");
        liblunatix::ipc::asid::asid_assign(CADDR_ASID_CONTROL, task_caps.vspace).unwrap();

        // load a stack for the child task
        log::debug!("mapping stack space for the new task");
        const TASK_STACK_LOW: usize = 0x5_0000_0000;
        liblunatix::ipc::page::map_page(
            task_caps.stack_page,
            task_caps.vspace,
            CADDR_MEM,
            TASK_STACK_LOW,
            MapFlags::READ | MapFlags::WRITE,
        )
        .unwrap();

        // load the elf content
        log::debug!("loading {path:?} elf code");
        let elf_binary = ElfBinary::new(&file_bin).unwrap();
        let mut elf_loader =
            LunatixElfLoader::new(CADDR_MEM, CADDR_VSPACE, task_caps.vspace, 0x31_0000_0000);
        elf_binary.load(&mut elf_loader).unwrap();
        elf_loader.remap_to_target_vspace();

        // setting task start params
        liblunatix::ipc::task::task_assign_control_registers(
            task_caps.task,
            elf_binary.entry_point() as usize,
            TASK_STACK_LOW + 4096,
            0x0,
            0x0,
        )
        .unwrap();

        task_caps
    }

    fn make_task_caps(&self) -> TaskCaps {
        let task = alloc_caddr();
        liblunatix::ipc::mem::derive(CADDR_MEM, task, CapabilityVariant::Task, None).unwrap();

        let cspace = alloc_caddr();
        liblunatix::ipc::mem::derive(CADDR_MEM, cspace, CapabilityVariant::CSpace, Some(2))
            .unwrap();
        liblunatix::ipc::task::task_assign_cspace(cspace, task).unwrap();

        let vspace = alloc_caddr();
        liblunatix::ipc::mem::derive(CADDR_MEM, vspace, CapabilityVariant::VSpace, None).unwrap();
        liblunatix::ipc::task::task_assign_vspace(vspace, task).unwrap();

        let stack_page = alloc_caddr();
        liblunatix::ipc::mem::derive(CADDR_MEM, stack_page, CapabilityVariant::Page, None).unwrap();

        TaskCaps {
            task,
            cspace,
            vspace,
            stack_page,
        }
    }

    fn destroy_task_caps(&self, caps: TaskCaps) {
        liblunatix::syscalls::destroy(caps.stack_page).unwrap();
        liblunatix::syscalls::destroy(caps.cspace).unwrap();
        liblunatix::syscalls::destroy(caps.task).unwrap();
        liblunatix::syscalls::destroy(caps.vspace).unwrap();
    }
}

impl Command for EndpointEcho {
    fn get_name(&self) -> &'static str {
        "endpoint_echo"
    }

    fn get_summary(&self) -> &'static str {
        "test endpoints by running the endpoint_echo binary"
    }

    fn execute(&self, _args: &str) -> Result<(), &'static str> {
        let server = self.load_binary("echo_srv");
        let client = self.load_binary("echo_client");

        log::info!("creating endpoint");
        let endpoint_addr = alloc_caddr();
        liblunatix::ipc::mem::derive(CADDR_MEM, endpoint_addr, CapabilityVariant::Endpoint, None)
            .unwrap();

        log::info!("copying endpoint copies into tasks");
        liblunatix::syscalls::copy(
            endpoint_addr,
            CAddr::builder()
                .part(server.cspace.raw(), CSPACE_BITS)
                .part(1, 1)
                .finish(),
        )
        .unwrap();
        liblunatix::syscalls::copy(
            endpoint_addr,
            CAddr::builder()
                .part(client.cspace.raw(), CSPACE_BITS)
                .part(1, 1)
                .finish(),
        )
        .unwrap();

        log::info!("executing server and client tasks");
        let mut sched = Scheduler::new([server.task, client.task].into_iter());
        sched.run_schedule();

        // TODO Cleanup the task objects

        Ok(())
    }
}
