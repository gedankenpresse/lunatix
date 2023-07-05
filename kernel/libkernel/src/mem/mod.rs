//! Memory management definitions
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
//! | `0xFFFFFFC000000000` | `0xFFFFFFEFFFFFFFFF` | 64 GB | direct mapping of all physical memory
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

/// The virtual memory address at which userspace tasks are mapped
///
/// See the [module documentation](super::mem) for an explanation of this value.
pub const VIRT_MEM_USER_START: usize = 0x0;

/// The last virtual memory address at which userspace tasks are mapped.
///
/// See the [module documentation](super::mem) for an explanation of this value.
pub const VIRT_MEM_USER_END: usize = 0x0000003fffffffff;

/// The virtual memory address at which physical memory starts being mapped.
///
/// See the [module documentation](super::mem) for an explanation of this value.
pub const VIRT_MEM_PHYS_MAP_START: usize = 0xFFFFFFC000000000;

/// The last virtual memory address at which physical memory is mapped.
///
/// See the [module documentation](super::mem) for an explanation of this value.
pub const VIRT_MEM_PHYS_MAP_END: usize = 0xFFFFFFEFFFFFFFFF;

/// The virtual memory address at which the kernel binary is mapped and where the kernel stack is located
///
/// See the [module documentation](super::mem) for an explanation of this value.
pub const VIRT_MEM_KERNEL_START: usize = 0xFFFFFFFF00000000;

/// The virtual memory address at which the kernel memory ends.
///
/// See the [module documentation](super::mem) for an explanation of this value.
pub const VIRT_MEM_KERNEL_END: usize = 0xFFFFFFFFFFFFFFFF;

/// How large each memory page is
///
/// This effects the alignment and sizes of some data structures that directly interface with the CPU e.g. PageTables
pub const PAGESIZE: usize = 4096;

/// Type definition for a slice of bytes that is exactly one page large
#[repr(C, align(4096))]
pub struct MemoryPage([u8; PAGESIZE]);
