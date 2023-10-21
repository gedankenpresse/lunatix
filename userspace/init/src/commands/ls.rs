use librust::println;

use crate::FS;

use super::Command;

pub struct Ls;

impl Command for Ls {
    fn get_name(&self) -> &'static str {
        "ls"
    }

    fn get_summary(&self) -> &'static str {
        "list directory"
    }

    fn execute(&self, args: &str) -> Result<(), &'static str> {
        let mut p9 = FS.0.borrow_mut();
        let p9 = p9.as_mut().unwrap();
        let mut dir_reader = p9.read_dir().unwrap();
        while let Some(entry) = dir_reader.read_entry() {
            println!("{}", entry.name);
        }
        Ok(())
    }
}
