//! Data-Structures and Definitions for Sv39 Virtual-Addressing
//!
//! # Virtual Addressing Basics
//!
//! Sv39 implementations support a 39-bit virtual address space, divided into 4 KiB pages.
//! An Sv39 address is partitioned as shown in the below figure.
//!
//! ```text
//! 38           30 29          21 20          12 11            0
//! ┌──────────────┬──────────────┬──────────────┬───────────────┐
//! │    VPN[2]    │    VPN[1]    │    VPN[0]    │  page offset  │
//! └──────────────┴──────────────┴──────────────┴───────────────┘
//!      9bits          9bits          9bits           12bits
//!                      Sv39 Virtual Address
//! ```
//!
//! This virtual address is translated into a physical address by transforming the VPN (virtual page number) segments
//! into PPN (physical page number) segments via a three-level page table hierarchy.
//! The 12-bit page offset is untranslated and carried over into the physical address.
//!
//! ```text
//! 55                   30 29          21 20          12 11            0
//! ┌──────────────────────┬──────────────┬──────────────┬───────────────┐
//! │        PPN[2]        │    PPN[1]    │    PPN[0]    │  page offset  │
//! └──────────────────────┴──────────────┴──────────────┴───────────────┘
//!          26bits             9bits          9bits           12bits
//!                      Sv39 Phyiscal Address
//! ```
//!
//! Virtual addresses, which are 64 bits, must have bits 63–39 all equal to bit 38, or else a page-fault exception will occur.
//!
//! # Addressing Schemes
//! For 64-bit RISCV multiple virtual memory systems are defined to relieve the tension between providing
//! a large address space and minimizing address-translation cost. For many systems, 512 GiB of
//! virtual-address space is ample, and so Sv39 suffices. Sv48 increases the virtual address space
//! to 256 TiB, but increases the physical memory capacity dedicated to page tables, the latency
//! of page-table traversals, and the size of hardware structures that store virtual addresses. Sv57
//! increases the virtual address space, page table capacity requirement, and translation latency even
//! further.
//!
//! TODO The below definitions are not defined by the hardware but instead are kernel dependent. We should move them into the kernel.
//!
//! ## Virtual Address Regions
//!
//! This kernel is currently hardcoded for RiscV Sv39 virtual addressing using the following
//! memory regions:
//!
//! | VAddr Start | VAddr End | Size | Usage |
//! | :---------- | :-------- | :--: | ----- |
//! | | | | **Per user context virtual memory** |
//! | `0x0000000000000000` | `0x0000003fffffffff` | 256 GB | userspace virtual memory
//! | | | | **Misc** |
//! | `0x0000004000000000` | `0xFFFFFFBFFFFFFFFF` | ~16M TB | unusable addresses
//! | | | | **Kernel-space virtual memory. Shared between all user contexts** |
//! | `0xFFFFFFC000000000` | `0xFFFFFFCFFFFFFFFF` | 64 GB | direct mapping of all physical memory
//! | ... | ... | ... | currently unused
//! | `0xFFFFFFFF00000000` | `0xFFFFFFFFFFFFFFFF` | 4 GB | Kernel
//!
//! ## Reasoning
//!
//! The above split between memory regions were chosen because:
//!
//! - The RiscV spec requires the virtual address bits 63-39 be equal to bit 38.
//!   This results in the large chunk of unusable addresses.
//! - The kernel regularly requires accessing physical addresses.
//!   To avoid switching virtual addressing on and off in these cases, the physical memory
//!   is directly mapped to virtual addresses.
//!   Since this is done by the kernel, translating physical to kernel-mapped addresses is easy.
//! - Because the kernel is being executed while virtual addressing is turned on, its code, data and other ELF content
//!   needs to be available through virtual addresses.
//!   For this, the kernel ELF binary is placed at the very last usable addresses.
//!

mod paddr;
mod page_table_entry;
mod page_tables;
pub mod ptrs;
mod vaddr;

use core::ops::{Deref, DerefMut};
pub use paddr::*;
pub use paddr::*;
pub use page_table_entry::*;
pub use page_tables::*;
pub use vaddr::*;

/// How large each page in the memory of a riscv board is.
///
/// This effects the alignment and sizes of some data structures that directly interface with the CPU e.g. PageTables
pub const PAGESIZE: usize = 4096;

/// Type definition for a slice of bytes that is exactly one page large and aligned to it as well
#[derive(Copy, Clone, Eq, PartialEq)]
#[repr(C, align(4096))]
pub struct MemoryPage([u8; PAGESIZE]);

impl Deref for MemoryPage {
    type Target = [u8; PAGESIZE];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for MemoryPage {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Default for MemoryPage {
    fn default() -> Self {
        Self([0u8; PAGESIZE])
    }
}

/// The virtual memory address at which userspace tasks are mapped
///
/// See the [module documentation](super::mem) for an explanation of this value.
#[deprecated(note = "put this into kernel")]
pub const VIRT_MEM_USER_START: usize = 0x0;

/// The last virtual memory address at which userspace tasks are mapped.
///
/// See the [module documentation](super::mem) for an explanation of this value.
#[deprecated(note = "put this into kernel")]
pub const VIRT_MEM_USER_END: usize = 0x0000003fffffffff;

/// The virtual memory address at which physical memory starts being mapped.
///
/// See the [module documentation](super::mem) for an explanation of this value.
#[deprecated(note = "put this into kernel")]
pub const VIRT_MEM_PHYS_MAP_START: usize = 0xFFFFFFC000000000;

/// The last virtual memory address at which physical memory is mapped.
///
/// See the [module documentation](super::mem) for an explanation of this value.
#[deprecated(note = "put this into kernel")]
pub const VIRT_MEM_PHYS_MAP_END: usize = 0xFFFFFFCFFFFFFFFF;

/// The virtual memory address at which the kernel binary is mapped and where the kernel stack is located
///
/// See the [module documentation](super::mem) for an explanation of this value.
#[deprecated(note = "put this into kernel")]
pub const VIRT_MEM_KERNEL_START: usize = 0xFFFFFFFF00000000;

/// The virtual memory address at which the kernel memory ends.
///
/// See the [module documentation](super::mem) for an explanation of this value.
#[deprecated(note = "put this into kernel")]
pub const VIRT_MEM_KERNEL_END: usize = 0xFFFFFFFFFFFFFFFF;
