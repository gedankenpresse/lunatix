use librust::print;

use crate::{read::Reader, FS};

use super::Command;

pub struct Cat;

impl Command for Cat {
    fn get_name(&self) -> &'static str {
        "cat"
    }

    fn get_summary(&self) -> &'static str {
        "read a file"
    }

    fn execute(&self, args: &str) -> Result<(), &'static str> {
        let path = args;
        let mut p9 = FS.0.borrow_mut();
        let p9 = p9.as_mut().unwrap();
        {
            let mut buf = [0u8; 128];
            let mut file = p9.read_file(&[path]).unwrap();
            while let Ok(bytes) = file.read(&mut buf) {
                if bytes == 0 {
                    break;
                }
                for &b in &buf[0..bytes] {
                    print!("{}", b as char);
                }
            }
        }
        Ok(())
    }
}
