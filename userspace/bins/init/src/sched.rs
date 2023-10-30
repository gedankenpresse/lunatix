use alloc::collections::VecDeque;
use liblunatix::prelude::CAddr;
use liblunatix::syscall_abi::yield_to::TaskStatus;

#[derive(Debug, Eq, PartialEq)]
pub struct Scheduler {
    tasks: VecDeque<CAddr>,
}

impl Scheduler {
    pub fn new(tasks: impl Iterator<Item = CAddr>) -> Self {
        Self {
            tasks: VecDeque::from_iter(tasks),
        }
    }

    pub fn run_schedule(&mut self) {
        while let Some(task) = self.tasks.pop_front() {
            log::debug!(
                "running schedule with {} tasks in round-robin until all are exited",
                self.tasks.len()
            );
            match liblunatix::yield_to(task).unwrap() {
                TaskStatus::DidExecute => {
                    self.tasks.push_back(task);
                }
                TaskStatus::Blocked => {
                    self.tasks.push_back(task);
                }
                TaskStatus::AlreadyRunning => panic!("task {task} is already running. WTF"),
                TaskStatus::Exited => {
                    log::debug!("task {task} exited")
                }
            }
        }
    }
}
