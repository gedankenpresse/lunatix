use core::str::FromStr;
use klog::println;
use log::Level;

/// Boot arguments that were specified by the user e.g. when starting QEMU
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct UserArgs {
    /// maximum log level that should be emitted
    pub log_level: log::Level,
}

impl UserArgs {
    pub fn from_str(args: &str) -> Self {
        let mut log_level = crate::DEFAULT_LOG_LEVEL;

        for arg in args.split(" ") {
            if let Some(level_arg) = arg.strip_prefix("log-level=") {
                log_level = Level::from_str(level_arg).expect("Could not parse log_level");
            }

            if arg == "help" || arg == "--help" {
                println!("Lunatix Kernel/OS");
                println!();
                println!("This kernel and operating system is a hobby project with the goal of creating a capability based kernel and accompanying operating system.");
                println!();
                println!("Parameters are always space-separated and either a key=value pair or have an effect when the key is present at all.");
                println!("Supported Kernel Parameter:");
                println!("  help                   Aborts the boot process and shows this help message instead.");
                println!("  log-level=LEVEL        Specifies the maximum log-level of kernel related logging. Can be one of ERROR, WARN, INFO, DEBUG, TRACE (case insensitive).");

                sbi::system_reset::system_reset(
                    sbi::system_reset::ResetType::Shutdown,
                    sbi::system_reset::ResetReason::NoReason,
                )
                .expect("Could not shutdown system after --help");
                unreachable!()
            }
        }

        Self { log_level }
    }
}

impl Default for UserArgs {
    fn default() -> Self {
        Self {
            log_level: crate::DEFAULT_LOG_LEVEL,
        }
    }
}
