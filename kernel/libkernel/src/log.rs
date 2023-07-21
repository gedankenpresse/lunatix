//! A logging implementation which uses an OpenSBI syscall to print characters
use core::fmt::Write;

use crate::print::KernelWriter;
use log::{Level, Log, Metadata, Record, SetLoggerError};

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

impl Log for KernelLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.max_log_level
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            KernelWriter {}
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
