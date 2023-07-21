use core::fmt;
use core::fmt::Write;

/// Dummy struct that makes converting [`fmt::Arguments`] easier to convert to strings
/// by offloading that to the [`Write`] trait.
pub struct SbiWriter {}

impl Write for SbiWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        // call into sbi firmware to write a each character to its output console
        for &char in s.as_bytes() {
            sbi::legacy::console_putchar(char);
        }
        Ok(())
    }
}
