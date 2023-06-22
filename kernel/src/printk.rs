use core::fmt;
use core::fmt::Write;

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::printk::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

/// Dummy struct that makes converting [`fmt::Arguments`] easier to convert to strings
/// by offloading that to the [`Write`] trait.
struct SbiWriter {}

impl Write for SbiWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        // call into sbi firmware to write a each character to its output console
        for &char in s.as_bytes() {
            sbi::legacy::console_putchar(char);
        }
        Ok(())
    }
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    SbiWriter {}.write_fmt(args).unwrap();
}
