use super::Command;
use liblunatix::prelude::syscall_abi::system_reset::{ResetReason, ResetType};

pub struct Shutdown;

impl Command for Shutdown {
    fn get_name(&self) -> &'static str {
        "shutdown"
    }

    fn get_summary(&self) -> &'static str {
        "shut down the system"
    }

    fn execute(&self, _args: &str) -> Result<(), &'static str> {
        liblunatix::syscalls::system_reset(ResetType::Shutdown, ResetReason::NoReason);
    }
}
