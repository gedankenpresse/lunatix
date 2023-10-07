use librust::println;
use librust::syscall_abi::system_reset::{ResetReason, ResetType};

use crate::second_task::SecondTask;

pub const KNOWN_COMMANDS: [&dyn Command; 4] = [&Help, &Shutdown, &Echo, &SecondTask];

pub trait Command {
    /// Get the name of the command
    fn get_name(&self) -> &'static str;

    /// Get the summary of this command
    fn get_summary(&self) -> &'static str;

    /// Execute the command with the given argument string
    fn execute(&self, args: &str) -> Result<(), ()>;
}

pub struct Shutdown;

impl Command for Shutdown {
    fn get_name(&self) -> &'static str {
        "shutdown"
    }

    fn get_summary(&self) -> &'static str {
        "shut down the system"
    }

    fn execute(&self, _args: &str) -> Result<(), ()> {
        librust::system_reset(ResetType::Shutdown, ResetReason::NoReason);
    }
}

pub struct Echo;

impl Command for Echo {
    fn get_name(&self) -> &'static str {
        "echo"
    }

    fn get_summary(&self) -> &'static str {
        "echo the input back to the user"
    }

    fn execute(&self, args: &str) -> Result<(), ()> {
        println!("{}", args);
        Ok(())
    }
}

pub struct Help;

impl Command for Help {
    fn get_name(&self) -> &'static str {
        "help"
    }

    fn get_summary(&self) -> &'static str {
        "print the list of commands"
    }

    fn execute(&self, _args: &str) -> Result<(), ()> {
        println!("Known Commands: ");
        for cmd in KNOWN_COMMANDS {
            println!("\t- {: <12} {}", cmd.get_name(), cmd.get_summary());
        }
        Ok(())
    }
}
