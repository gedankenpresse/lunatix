//! Type definitions, functions and constants for working with riscv virtual addresses
//!
//! Most documentation is located on the [`VAddr`] type definition.

use static_assertions::const_assert_eq;

/// Type alias for virtual addresses.
///
/// This is used by functions that explicitly interpret addresses as virtual ones.
///
/// # Memory Layout
/// Virtual addresses are backed by 64 bits of information which is partitioned into a
/// *Virtual Page Number (VPN)* and a *Page Offset* as shown in the figure below.
///
/// Sometimes (e.g. when walking a pagetable) it is necessary to process 9 bits of the VPN at a time.
/// For this purpose, the VPN is further partitioned into segments labeled `0`, `1` and `2`.
///
/// ```text
/// 38           30 29          21 20          12 11            0
/// ┌──────────────┬──────────────┬──────────────┬───────────────┐
/// │    VPN[2]    │    VPN[1]    │    VPN[0]    │  page offset  │
/// └──────────────┴──────────────┴──────────────┴───────────────┘
///      9bits          9bits          9bits           12bits
///                      Sv39 Virtual Address
/// ```
///
/// # Working with VAddrs
///
/// This module also contains constants relating to the memory layout described above.
///
/// Usage of these constants is demonstrated in the examples below.
///
/// ## Example 1
///
/// This example demonstrates how provided _\_MASK_ constants can be used to access different
/// parts of a VAddr:
///
/// ```rust
/// # use riscv::mem::vaddr::*;
/// // a vaddr with its VPN set to 0b11111… and page offset to 1
/// let vaddr: VAddr = 0x7FFFFFF001;
///
/// let vpn = vaddr & VPN_MASK;
/// let page_offset = vaddr & PAGE_OFFSET_MASK;
/// assert_eq!(vpn, 0x7FFFFFF000);
/// assert_eq!(page_offset, 1);
/// ```
pub type VAddr = u64;

/// The number of bits that are utilized by VAddrs
pub const VADDR_BITS: u64 = 39;
/// A bitmask that has `1` in all bits that are utilized by VAddrs
pub const VADDR_MASK: u64 = (1 << VADDR_BITS) - 1;

/// The number of bits that make up the page offset part of a VAddr
pub const PAGE_OFFSET_BITS: u64 = 12;
/// A bitmask that has `1` in all bits associated to a VAddrs page offset part
pub const PAGE_OFFSET_MASK: u64 = (1 << PAGE_OFFSET_BITS) - 1;

/// The number of bits that make up the VPN part of a VAddr
pub const VPN_BITS: u64 = 27;
/// The distance from a VAddrs least-significant-bit to the least-significant-bit of a VPN
pub const VPN_OFFSET: u64 = PAGE_OFFSET_BITS;
/// A bitmask that has `1` in all bits associated to a VAddrs VPN part
pub const VPN_MASK: u64 = ((1 << VPN_BITS) - 1) << VPN_OFFSET;

/// The number of bits that make up each segment of a VAddrs VPN
pub const VPN_SEGMENT_BITS: u64 = 9;
const VPN_SEGMENT_MASK: u64 = (1 << VPN_SEGMENT_BITS) - 1;

/// A bitmask that has `1` in all bits associated to a VAddrs _VPN[0]_ part
pub const VPN0_MASK: u64 = VPN_SEGMENT_MASK << VPN0_OFFSET;
/// The distance from a VAddrs least-significant-bit to the least-significant-bit of _VPN[0]_
pub const VPN0_OFFSET: u64 = VPN_OFFSET;

/// A bitmask that has `1` in all bits associated to a VAddrs _VPN[1]_ part
pub const VPN1_MASK: u64 = VPN_SEGMENT_MASK << VPN1_OFFSET;
/// The distance from a VAddrs least-significant-bit to the least-significant-bit of _VPN[1]_
pub const VPN1_OFFSET: u64 = VPN0_OFFSET + VPN_SEGMENT_BITS;

/// A bitmask that has `1` in all bits associated to a VAddrs _VPN[2]_ part
pub const VPN2_MASK: u64 = VPN_SEGMENT_MASK << VPN2_OFFSET;
/// The distance from a VAddrs least-significant-bit to the least-significant-bit of _VPN[2]_
pub const VPN2_OFFSET: u64 = VPN1_OFFSET + VPN_SEGMENT_BITS;

const_assert_eq!(PAGE_OFFSET_BITS, super::paddr::PAGE_OFFSET_BITS);
const_assert_eq!(VPN_BITS, VPN_SEGMENT_BITS * 3);
const_assert_eq!(VADDR_BITS, PAGE_OFFSET_BITS + VPN_BITS);

/// Get the VPN (virtual page number) segments from a virtual address
///
/// Note that each segment is shifted to be interpreted individually.
/// The segments cannot simply be ORed together to reconstruct the input VPN.
#[inline]
pub fn vpn_segments(vaddr: VAddr) -> [u64; 3] {
    [
        (vaddr & VPN0_MASK) >> VPN0_OFFSET,
        (vaddr & VPN1_MASK) >> VPN1_OFFSET,
        (vaddr & VPN2_MASK) >> VPN2_OFFSET,
    ]
}
