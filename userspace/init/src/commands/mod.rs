mod echo;
mod identify;
mod second_task;
mod shutdown;

pub use echo::Echo;
pub use identify::Identify;
pub use second_task::SecondTask;
pub use shutdown::Shutdown;

pub trait Command {
    /// Get the name of the command
    fn get_name(&self) -> &'static str;

    /// Get the summary of this command
    fn get_summary(&self) -> &'static str;

    /// Execute the command with the given argument string
    fn execute(&self, args: &str) -> Result<(), ()>;
}
