use core::str::FromStr;

use librust::syscall_abi::CAddr;

use super::Command;

pub struct Destroy;

struct DestroyArgs {
    addr: CAddr,
}

impl FromStr for DestroyArgs {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let caddr = s.trim().split(" ").next().ok_or("failed to read caddr")?;
        let caddr = caddr
            .parse::<usize>()
            .map_err(|_| "failed to parse caddr")?;
        Ok(Self { addr: caddr })
    }
}

impl Command for Destroy {
    fn get_name(&self) -> &'static str {
        "destroy"
    }

    fn get_summary(&self) -> &'static str {
        "destory system call"
    }

    fn execute(&self, args: &str) -> Result<(), &'static str> {
        let DestroyArgs { addr } = DestroyArgs::from_str(args)?;
        let Ok(_) = librust::destroy(addr) else {
            return Err("syscall failed");
        };
        Ok(())
    }
}
