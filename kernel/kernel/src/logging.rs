use core::fmt;
use core::fmt::Write;
use log::{Level, LevelFilter, Log, Metadata, Record, SetLoggerError};

#[derive(Debug)]
pub struct KernelLogger {
    pub max_log_level: Level,
}

impl KernelLogger {
    pub const fn new(max_log_level: Level) -> KernelLogger {
        KernelLogger { max_log_level }
    }

    pub fn install(&'static self) -> Result<(), SetLoggerError> {
        log::set_logger(self).map(|_| log::set_max_level(self.max_log_level.to_level_filter()))
    }
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

impl Log for KernelLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.max_log_level
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            SbiWriter {}
                .write_fmt(format_args!(
                    "{} - {}: {}\n",
                    record.level(),
                    record.target(),
                    record.args(),
                ))
                .expect("Could not write log message to Sbi")
        }
    }

    fn flush(&self) {}
}
