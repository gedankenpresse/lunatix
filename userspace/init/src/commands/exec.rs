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
        let mut v = vec![];
        let mut buf = [0u8; 256];
        let mut reader = p9.read_file(&[path]).unwrap();
        while let Ok(bytes) = reader.read(buf.as_mut()) {
            println!("read {}", bytes);
            let read = &buf[0..bytes];
            v.extend_from_slice(read);
        }
        println!("{:?}", &v);
        Ok(())
    }
}
