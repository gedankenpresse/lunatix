use liblunatix::println;
use log::{Level, Log, Metadata, Record, SetLoggerError};

pub struct Logger {
    pub max_log_level: Level,
}

impl Logger {
    pub const fn new(max_log_level: Level) -> Logger {
        Logger { max_log_level }
    }

    pub fn install(&'static self) -> Result<(), SetLoggerError> {
        log::set_logger(self).map(|_| log::set_max_level(self.max_log_level.to_level_filter()))
    }
}

impl Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.max_log_level
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
            println!("{}  {}: {}", level_moji, record.target(), record.args(),);
        }
    }

    fn flush(&self) {}
}
