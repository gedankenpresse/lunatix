//! ABI definitions for calling into the kernel and interpreting its results
//!
//! # Syscall Overview
//!
//! Currently, the following syscalls are defined:
//!
//! | Syscall | Number | Arguments | Return | Summary |
//! |--------|:-----------:|-----------|--------|---------|
//! | [debug_log](debug_log::DebugLog) | *0* | [DebugLogArgs](debug_log::DebugLogArgs) | [DebugLogReturn](debug_log::DebugLogReturn) | Put a c-string on the kernels attached serial console |
//! | [debug_putc](debug_putc::DebugPutc) | *1* | [DebugPutcArgs](debug_putc::DebugPutcArgs) | [DebugPutcReturn](debug_putc::DebugPutcReturn) | Put a signel character on the kerneles attached serial console |
//! | [identify](identify::Identify) | *3* | [IdentifyArgs](identify::IdentifyArgs) | [IdentifyReturn](identify::IdentifyReturn) | Identify the capability stored at a given CAddr |
//! | [alloc_page](alloc_page::AllocPage) | *4* | [AllocPageArgs](alloc_page::AllocPageArgs) | [AllocPageReturn](alloc_page::AllocPageReturn) | Allocate a single page from a memory capability |
//! | [map_page](map_page::MapPage) | *5* | [MapPageArgs](map_page::MapPageArgs) | [MapPageReturn](map_page::MapPageReturn) | Map a page into a tasks vspace |
//! | [assign_ipc_buffer](assign_ipc_buffer::AssignIpcBuffer) | *6* | [AssignIpcBufferArgs](assign_ipc_buffer::AssignIpcBufferArgs) | [AssignIpcBufferReturn](assign_ipc_buffer::AssignIpcBufferReturn) | Assign an already allocated page to be used as IPC buffer |
//! | [derive_from_mem](derive_from_mem::DeriveFromMem) | *7* | [DeriveFromMemArgs](derive_from_mem::DeriveFromMemArgs) | [DeriveFromMemReturn](derive_from_mem::DeriveFromMemReturn) | Derive a new capability from a memory capability |
//! | [task_assign_cspace](task_assign_cspace::TaskAssignCSpace) | *8* | [AssignCSpaceArgs](task_assign_cspace::TaskAssignCSpaceArgs) | [AssignCSpaceReturn](task_assign_cspace::TaskAssignCSpaceReturn) | Assign a cspace to a task |
//! | [task_assign_vspace](task_assign_vspace::TaskAssignVSpace) | *9* | [AssignVSpaceArgs](task_assign_vspace::TaskAssignVSpaceArgs) | [AssignVSpaceReturn](task_assign_cspace::AssignVSpaceReturn) | Assign a vspace to a task |
//!

#![no_std]
#![allow(clippy::enum_clike_unportable_variant)]

use crate::generic_return::GenericReturn;

pub mod assign_ipc_buffer;
pub mod debug_log;
pub mod debug_putc;
pub mod derive_from_mem;
pub mod generic_return;
pub mod identify;
pub mod inspect_derivation_tree;
pub mod map_page;
pub mod task_assign_cspace;
pub mod task_assign_vspace;

/// A type alias for explicitly marking a capability address in type signatures.
pub type CAddr = usize;

/// A trait for binding a syscall number to its specific argument and return type.
pub trait SyscallBinding {
    /// The syscall number which identifies this syscall.
    const SYSCALL_NO: usize;

    /// The type that is used to encode the syscalls arguments.
    ///
    /// These are usually syscall specific but are required to be represent themselves as `RawSyscallArgs` since that
    /// is what is written to the CPUs registers when the syscall is executed.
    /// Accordingly, the kernel needs to be able to reconstruct the arguments by reading the registers and thus,
    /// a backwards conversion from `RawSyscallArgs` must also be possible.
    type CallArgs: TryFrom<RawSyscallArgs> + Into<RawSyscallArgs>;

    /// The type that is used to encode the syscalls result.
    ///
    /// The syscall result is usually specific to a syscall but must be a superset of `GenericReturn` which is why
    /// conversion to and from `GenericReturn` must be possible.
    type Return: TryFrom<RawSyscallReturn> + Into<RawSyscallReturn> + Into<GenericReturn>;
}

/// A trait binding a syscall to a `repr(C)` type which is expected to be put into the tasks IPC buffer when calling it.
pub trait IpcArgsBinding: SyscallBinding {
    type IpcArgs;
}

/// A trait binding a syscall to a `repr(C)` type which the kernel puts into the tasks IPC buffer as a result when
/// called.
pub trait IpcReturnBinding: SyscallBinding {
    type IpcReturn;
}

/// The arguments to a syscall as they are encoded in the CPUs registers.
pub type RawSyscallArgs = [usize; 7];

/// The return value of a syscall as they are encoded in the CPUs registers.
pub type RawSyscallReturn = [usize; 2];
