use alloc::collections::VecDeque;
use alloc::vec::Vec;
use liblunatix::prelude::syscall_abi::yield_to::TaskStatus;
use liblunatix::prelude::CAddr;

pub struct Scheduler {
    tasks: VecDeque<Task>,
}

pub struct Task {
    /// The CAddr of the task itself
    caddr: CAddr,
    /// The CAddr of the tasks cspace
    cspace: CAddr,
    /// The CAddr of the tasks vspace
    vspace: CAddr,
    /// The pages which are mapped into the tasks address space
    mapped_pages: Vec<CAddr>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            tasks: VecDeque::with_capacity(8),
        }
    }

    pub fn add_to_schedule(&mut self, task: Task) {
        self.tasks.push_back(task);
    }

    /// Run the next task once
    pub fn execute(&mut self) {
        let Some(task) = self.tasks.pop_front() else {
            return;
        };

        match liblunatix::syscalls::yield_to(task.caddr).unwrap() {
            TaskStatus::DidExecute | TaskStatus::Blocked => {
                log::debug!("executed task {:?} successfully", task.caddr);
                self.tasks.push_back(task);
            }
            TaskStatus::AlreadyRunning => panic!("scheduled task is already running"),
            TaskStatus::Exited => {
                todo!("cleanup the tasks resources")
            }
        }
    }
}
