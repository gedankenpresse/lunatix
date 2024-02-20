//! Type definitions, functions and constants for working with riscv virtual addresses
//!
//! Most documentation is located on the [`PAddr`] type definition.

use static_assertions::const_assert_eq;

/// Type alias for physical addresses.
///
/// This is used by functions that explicitly interpret addresses as virtual ones.
///
/// # Memory Layout
/// Physical addresses are backed by 64 bits of information which is partitioned into a
/// *Physical Page Number (PPN)* and a *Page Offset* as shown in the figure below.
///
/// ```text
/// 55                   30 29          21 20          12 11            0
/// ┌──────────────────────┬──────────────┬──────────────┬───────────────┐
/// │        PPN[2]        │    PPN[1]    │    PPN[0]    │  page offset  │
/// └──────────────────────┴──────────────┴──────────────┴───────────────┘
///          26bits             9bits          9bits           12bits
///                      Sv39 Phyiscal Address
///```
///
/// # Working with PAddrs
///
/// This module also contains constants relating to the memory layout described above.
///
/// Usage of these constants is demonstrated in the examples below.
///
/// ## Example 1
///
/// TODO Write Examples
pub type PAddr = u64;

/// The number of bits that are utilized by PAddrs
pub const PADDR_BITS: u64 = 56;
/// A bitmask that has `1` in all bits that are utilized by PAddrs
pub const PADDR_MASK: u64 = (1 << PADDR_BITS) - 1;

/// The number of bits that make up the page offset part of a PAddr
pub const PAGE_OFFSET_BITS: u64 = 12;
/// A bitmask that has `1` in all bits associated to a PAddrs page offset part
pub const PAGE_OFFSET_MASK: u64 = (1 << PAGE_OFFSET_BITS) - 1;

/// The number of bits that make up the PPN part of a PAddr
pub const PPN_BITS: u64 = 44;
/// The distance from a PAddrs least-significant-bit to the least-significant-bit of a PPN
pub const PPN_OFFSET: u64 = PAGE_OFFSET_BITS;
/// A bitmask that has `1` in all bits associated to a PAddrs PPN part
pub const PPN_MASK: u64 = ((1 << PPN_BITS) - 1) << PPN_OFFSET;

/// The number of bits that make up a PAddrs _PPN[0]_ part
pub const PPN0_BITS: u64 = 9;
/// The distance from a PAddrs least-significant-bit to the least-significant-bit of a PAddrs _PPN[0] part
pub const PPN0_OFFSET: u64 = PPN_OFFSET;
/// A bitmask that has `1` in all bits associated to a PAddrs _PPN[0]_ part
pub const PPN0_MASK: u64 = ((1 << PPN0_BITS) - 1) << (PAGE_OFFSET_BITS);

/// The number of bits that make up a PAddrs _PPN[1]_ part
pub const PPN1_BITS: u64 = 9;
/// The distance from a PAddrs least-significant-bit to the least-significant-bit of a PAddrs _PPN[1] part
pub const PPN1_OFFSET: u64 = PPN0_OFFSET + PPN0_BITS;
/// A bitmask that has `1` in all bits associated to a PAddrs _PPN[1]_ part
pub const PPN1_MASK: u64 = ((1 << PPN1_BITS) - 1) << PPN1_OFFSET;

/// The number of bits that make up a PAddrs _PPN[2]_ part
pub const PPN2_BITS: u64 = 26;
/// The distance from a PAddrs least-significant-bit to the least-significant-bit of a PAddrs _PPN[2] part
pub const PPN2_OFFSET: u64 = PPN1_OFFSET + PPN1_BITS;
/// A bitmask that has `1` in all bits associated to a PAddrs _PPN[2]_ part
pub const PPN2_MASK: u64 = ((1 << PPN2_BITS) - 1) << PPN2_OFFSET;

const_assert_eq!(PPN_BITS, PPN0_BITS + PPN1_BITS + PPN2_BITS);
const_assert_eq!(PADDR_BITS, PPN_BITS + PAGE_OFFSET_BITS);

/// Get the PPN (physical page number) segments from a physical address
///
/// Note that each segment is shifted to be interpreted individually.
/// The segments cannot simply be ORed together to reconstruct the input PPN.
#[inline]
pub fn ppn_segments(paddr: PAddr) -> [u64; 3] {
    [
        (paddr & PPN0_MASK) >> PPN0_OFFSET,
        (paddr & PPN1_MASK) >> PPN1_OFFSET,
        (paddr & PPN2_MASK) >> PPN2_OFFSET,
    ]
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_paddr_ppn_segments() {
        let ppn0 = 0x1FF000;
        let ppn0_sgmt = ppn_segments(ppn0)[0];
        assert_eq!(ppn0_sgmt, ppn0 >> 12, "{:x} != {:x}", ppn0_sgmt, ppn0 >> 12);

        let ppn1 = 0x3FE00000;
        let ppn1_sgmt = ppn_segments(ppn1)[1];
        assert_eq!(ppn1_sgmt, ppn1 >> 21, "{:x} != {:x}", ppn1_sgmt, ppn1 >> 21);

        let ppn2 = 0xFFFFFFC0000000;
        let ppn2_sgmt = ppn_segments(ppn2)[2];
        assert_eq!(ppn2_sgmt, ppn2 >> 30, "{:x} != {:x}", ppn2_sgmt, ppn2 >> 30);
    }
}
