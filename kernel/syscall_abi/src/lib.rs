//! ABI definitions for calling into the kernel and interpreting its results
//!
//! # Syscall Overview
//!
//! Currently, the following syscalls are defined:
//!
//! | Struct | Syscall No. | Arguments | Return | Summary |
//! |--------|:-----------:|-----------|--------|---------|
//! | [DebugLog](debug_log::DebugLog) | *0* | [DebugLogArgs](debug_log::DebugLogArgs) | [DebugLogReturn](debug_log::DebugLogReturn) | Put a c-string on the kernels attached serial console |
//! | [DebugPutc](debug_putc::DebugPutc) | *1* | [DebugPutcArgs](debug_putc::DebugPutcArgs) | [DebugPutcReturn](debug_putc::DebugPutcReturn) | Put a signel character on the kerneles attached serial console |
//! | [Identify](identify::Identify) | *3* | [IdentifyArgs](identify::IdentifyArgs) | [IdentifyReturn](identify::IdentifyReturn) | Identify the capability stored at a given CAddr |
//!

#![no_std]
#![allow(clippy::enum_clike_unportable_variant)]

pub mod debug_log;
pub mod debug_putc;
pub mod generic_return;
pub mod identify;

/// A trait for binding a syscall number to its specific argument and return type.
pub trait SyscallBinding {
    /// The syscall number which identifies this syscall.
    const SYSCALL_NO: usize;

    type CallArgs: TryFrom<RawSyscallArgs> + Into<RawSyscallArgs>;

    type Return: TryFrom<RawSyscallReturn> + Into<RawSyscallReturn>;
}

/// The arguments to a syscall as they are encoded in the CPUs registers.
pub type RawSyscallArgs = [usize; 7];

/// The return value of a syscall as they are encoded in the CPUs registers.
pub type RawSyscallReturn = [usize; 2];
