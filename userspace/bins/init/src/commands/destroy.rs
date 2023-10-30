use super::{CAddrArg, Command, ToValue};

pub struct Destroy;

impl Command for Destroy {
    fn get_name(&self) -> &'static str {
        "destroy"
    }

    fn get_summary(&self) -> &'static str {
        "destory system call"
    }

    fn execute(&self, args: &str) -> Result<(), &'static str> {
        let CAddrArg { addr } = args.to_value()?;
        let Ok(_) = liblunatix::syscalls::destroy(addr) else {
            return Err("syscall failed");
        };
        Ok(())
    }
}
