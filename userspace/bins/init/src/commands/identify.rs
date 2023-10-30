use liblunatix::println;

use super::Command;

pub struct Identify;

impl Command for Identify {
    fn get_name(&self) -> &'static str {
        "id"
    }

    fn get_summary(&self) -> &'static str {
        "identify syscall"
    }

    fn execute(&self, args: &str) -> Result<(), &'static str> {
        let caddr = args
            .trim()
            .split(" ")
            .next()
            .ok_or("failed to read caddr")?;
        let caddr = caddr
            .parse::<usize>()
            .map_err(|_| "failed to parse caddr")?;
        println!("{:?}", liblunatix::identify(caddr));
        Ok(())
    }
}
