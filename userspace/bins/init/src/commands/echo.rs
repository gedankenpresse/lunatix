use liblunatix::println;

use super::Command;

pub struct Echo;

impl Command for Echo {
    fn get_name(&self) -> &'static str {
        "echo"
    }

    fn get_summary(&self) -> &'static str {
        "echo the input back to the user"
    }

    fn execute(&self, args: &str) -> Result<(), &'static str> {
        println!("{}", args);
        Ok(())
    }
}
