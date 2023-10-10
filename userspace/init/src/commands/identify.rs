use librust::println;

use super::Command;

pub struct Identify;

impl Command for Identify {
    fn get_name(&self) -> &'static str {
        "id"
    }

    fn get_summary(&self) -> &'static str {
        "identify syscall"
    }

    fn execute(&self, args: &str) -> Result<(), ()> {
        let caddr = args.trim().split(" ").next().ok_or(())?;
        let caddr = caddr.parse::<usize>().map_err(|_| ())?;
        println!("{:?}", librust::identify(caddr));
        Ok(())
    }
}
