mod copy;
mod destroy;
mod echo;
mod identify;
mod second_task;
mod shutdown;

pub use copy::Copy;
pub use destroy::Destroy;
pub use echo::Echo;
pub use identify::Identify;
use librust::syscall_abi::CAddr;
pub use second_task::SecondTask;
pub use shutdown::Shutdown;

pub trait Command {
    /// Get the name of the command
    fn get_name(&self) -> &'static str;

    /// Get the summary of this command
    fn get_summary(&self) -> &'static str;

    /// Execute the command with the given argument string
    fn execute(&self, args: &str) -> Result<(), &'static str>;
}

pub struct CAddrArg {
    pub addr: CAddr,
}

impl<A: FromArgs, B: FromArgs> FromArgs for (A, B) {
    fn parse<'a>(args: &'a str) -> Result<(Self, &'a str), &'static str>
    where
        Self: Sized,
    {
        let (a, rest) = A::parse(args)?;
        let (b, rest) = B::parse(rest)?;
        Ok(((a, b), rest))
    }
}

pub trait FromArgs {
    fn parse<'a>(args: &'a str) -> Result<(Self, &'a str), &'static str>
    where
        Self: Sized;
    fn from_args(args: &str) -> Result<Self, &'static str>
    where
        Self: Sized,
    {
        let (a, _rest) = Self::parse(args)?;
        Ok(a)
    }
}

pub trait ToValue<Value> {
    fn to_value(&self) -> Result<Value, &'static str>;
}

impl<V: FromArgs> ToValue<V> for &str {
    fn to_value(&self) -> Result<V, &'static str> {
        V::from_args(self)
    }
}

impl FromArgs for CAddrArg {
    fn parse(s: &str) -> Result<(Self, &str), &'static str> {
        let s = s.trim_start();
        match s.split_once(" ") {
            Some((a, rest)) => {
                let addr = a.parse().map_err(|_| "arg is not a number")?;
                Ok((Self { addr }, rest))
            }
            None => {
                let addr = s.parse().map_err(|_| "arg is not a number")?;
                Ok((Self { addr }, ""))
            }
        }
    }
}
