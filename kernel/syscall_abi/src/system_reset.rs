//! Definitions for the `system_reset` syscall.

use crate::{NoValue, RawSyscallArgs, SyscallBinding, SyscallResult};

pub struct SystemReset;

/// The reason for performing the reset
#[derive(Debug, Eq, PartialEq, Default)]
#[repr(u8)]
pub enum ResetReason {
    /// No reason for reset
    #[default]
    NoReason = 0,
    /// System failure
    SystemFailure = 1,
}

/// The type of reset to perform
#[derive(Debug, Eq, PartialEq, Default)]
#[repr(u8)]
pub enum ResetType {
    /// Shut down and power off the system
    #[default]
    Shutdown = 0,
    /// Power off all hardware and perform a cold reboot
    ColdReboot = 1,
    /// Reset the processor and some hardware to perform a warm reboot
    WarmReboot = 2,
}

#[derive(Debug, Eq, PartialEq, Default)]
pub struct SystemResetArgs {
    pub typ: ResetType,
    pub reason: ResetReason,
}

impl SyscallBinding for SystemReset {
    const SYSCALL_NO: usize = 16;
    type CallArgs = SystemResetArgs;
    type Return = SyscallResult<NoValue>;
}

impl From<SystemResetArgs> for RawSyscallArgs {
    fn from(value: SystemResetArgs) -> Self {
        [value.typ as usize, value.reason as usize, 0, 0, 0, 0, 0]
    }
}

impl From<RawSyscallArgs> for SystemResetArgs {
    fn from(value: RawSyscallArgs) -> Self {
        Self {
            typ: match value[0] {
                0 => ResetType::Shutdown,
                1 => ResetType::ColdReboot,
                2 => ResetType::WarmReboot,
                _ => panic!(),
            },
            reason: match value[1] {
                0 => ResetReason::NoReason,
                1 => ResetReason::SystemFailure,
                _ => panic!(),
            },
        }
    }
}
