use core::fmt;
use core::fmt::Write;

/// Dummy struct that makes converting [`fmt::Arguments`] easier to convert to strings
/// by offloading that to the [`Write`] trait.
pub struct KernelWriter {}

impl Write for KernelWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        todo!()
    }
}
