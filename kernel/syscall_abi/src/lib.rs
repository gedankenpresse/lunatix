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
//! | [task_assign_control_registers](task_assign_control_registers::TaskAssignControlRegisters) | *10* | [TaskAssignControlRegistersArgs](task_assign_control_registers::TaskAssignControlRegistersArgs) | [TaskAssignControlRegistersReturn](task_assign_control_registers::TaskAssignControlRegistersReturn) | Assign control reigsters like `pc` and `sp` to the task |
//! | [yield_to](yield_to::YieldTo) | *11* | [YieldToArgs](yield_to::YieldToArgs) | [YieldToReturn](yield_to::YieldToReturn) | Yield execution to another task |
//! | [yield](yield::Yield) | *12* | [YieldArgs](yield::YieldArgs) | [YieldReturn](yield::YieldReturn) | Yield execution back to the scheduler |
//! | [irq_control_claim](irq_control_claim::IrqControlClaim) | *13* | [IrqControlClaimArgs](irq_control_claim::IrqControlClaimArgs) | [NoValue](NoValue) | Claim the handling of a specific interrupt line |
//! | [wait_on](wait_on::WaitOn) | *14* | [WaitOnArgs](wait_on::WaitOnArgs) | `usize` | Wait on a notification until it is set with a value |
//! | [irq_complete](irq_complete::IrqComplete) | *15* | [IrqCompleteArgs](irq_complete::IrqCompleteArgs) | [NoValue](NoValue) | Mark the interrupt on an IRQ as completed |
//! | [system_reset](system_reset::SystemReset) | *16* | [SystemResetArgs](system_reset::SystemResetArgs) | [NoValue](NoValue) | Schedule a hardware reset |
//! | [map_devmem](map_devmem::MapDevmem) | *17* | [MapDevmemArgs](map_devmem::MapDevmemArgs) | [NoValue](NoValue) | Map Device Memory
//! | [send] | *18* |
//! | [destroy] | *19* |
//! | [copy] | *20* |
//! | [get_page_paddr] | *21* |
//! | [exit] | *22* |

#![no_std]
#![allow(clippy::enum_clike_unportable_variant)]

use core::usize;

pub mod assign_ipc_buffer;
pub mod debug;
mod errors;
pub mod exit;
pub mod identify;
pub mod inspect_derivation_tree;
mod ipc_tag;
pub mod send;
pub mod system_reset;
mod traits;
pub mod wait_on;
pub mod r#yield;
pub mod yield_to;

pub use errors::Error;
pub use ipc_tag::IpcTag;
pub use traits::*;

/// A type alias for explicitly marking a capability address in type signatures.
pub type CAddr = usize;
