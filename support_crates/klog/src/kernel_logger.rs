//! A logging implementation which uses an OpenSBI syscall to print characters
use core::fmt::Write;

use crate::print::KernelWriter;
use log::{Level, Log, Metadata, Record, SetLoggerError};

// TODO Improve updating the maximum log level
// Currently, only the global log:: filter is updated because that's easier that worrying about internal mutability of
// the KernelLogger struct which would be required when wanting to update internal state too

pub struct KernelLogger {
    pub initial_log_level: Level,
}

impl KernelLogger {
    pub const fn new(max_log_level: Level) -> KernelLogger {
        KernelLogger {
            initial_log_level: max_log_level,
        }
    }

    pub fn install(&'static self) -> Result<(), SetLoggerError> {
        log::set_logger(self).map(|_| log::set_max_level(self.initial_log_level.to_level_filter()))
    }

    pub fn update_log_level(&'static self, level: Level) {
        log::set_max_level(level.to_level_filter());
    }
}

impl Log for KernelLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let level_moji = match record.level() {
                Level::Error => "❌",
                Level::Warn => "⚠️",
                Level::Info => "ℹ️",
                Level::Debug => "🛠️",
                Level::Trace => "👣",
            };
            KernelWriter {}
                .write_fmt(format_args!(
                    "{}  {}: {}\n",
                    level_moji,
                    record.target(),
                    record.args(),
                ))
                .expect("Could not write log message to Sbi")
        }
    }

    fn flush(&self) {}
}
