use super::{CAddrArg, Command, ToValue};

pub struct Copy;

impl Command for Copy {
    fn get_name(&self) -> &'static str {
        "copy"
    }

    fn get_summary(&self) -> &'static str {
        "copy syscall"
    }

    fn execute(&self, args: &str) -> Result<(), &'static str> {
        let (CAddrArg { addr: source }, CAddrArg { addr: target }) = args.to_value()?;
        librust::copy(source, target).unwrap();
        Ok(())
    }
}
