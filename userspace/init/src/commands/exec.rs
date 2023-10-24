use alloc::vec;
use librust::println;

use crate::{read::Reader, FS};

use super::Command;

pub struct Exec;

impl Command for Exec {
    fn get_name(&self) -> &'static str {
        "exec"
    }

    fn get_summary(&self) -> &'static str {
        "execute a binary (currently only loads a binary)"
    }

    fn execute(&self, args: &str) -> Result<(), &'static str> {
        let path = args;
        let mut p9 = FS.0.borrow_mut();
        let p9 = p9.as_mut().unwrap();
        let mut reader = p9.read_file(&[path]).unwrap();
        let file_bin = reader.read_to_vec().unwrap();

        log::info!("successfully read binary from filesystem");

        todo!("actually run the second task");

        Ok(())
    }
}
