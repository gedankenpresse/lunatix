/// Type alias for physical addresses.
///
/// This is used by functions that explicitly interpret addresses as virtual ones.
pub type PAddr = u64;

pub(crate) const PAGE_OFFSET_BITS: u64 = 12;
pub(crate) const PAGE_OFFSET_MASK: u64 = (1 << PAGE_OFFSET_BITS) - 1;

pub(crate) const PPN0_BITS: u64 = 9;
pub(crate) const PPN0_MASK: u64 = ((1 << PPN0_BITS) - 1) << (PAGE_OFFSET_BITS);
pub(crate) const PPN1_BITS: u64 = 9;
pub(crate) const PPN1_MASK: u64 = ((1 << PPN1_BITS) - 1) << (PAGE_OFFSET_BITS + PPN0_BITS);
pub(crate) const PPN2_BITS: u64 = 26;
pub(crate) const PPN2_MASK: u64 =
    ((1 << PPN2_BITS) - 1) << (PAGE_OFFSET_BITS + PPN0_BITS + PPN1_BITS);
pub(crate) const PPN_MASK: u64 = PPN0_MASK | PPN1_MASK | PPN2_MASK;

/// Get the PPN (physical page number) segments from a physical address
#[inline]
pub fn paddr_ppn_segments(paddr: PAddr) -> [u64; 3] {
    [paddr & PPN0_MASK, paddr & PPN1_MASK, paddr & PPN2_MASK]
}

/// Get the physical page number encoded in a physical address
///
/// The phyiscal page number is the same as the paddr but has all page offset bits set to zero.
#[inline]
pub fn paddr_ppn(paddr: PAddr) -> u64 {
    paddr & PPN_MASK
}

/// Get the page offset from a physical address
#[inline]
pub fn paddr_page_offset(paddr: PAddr) -> u64 {
    paddr & PAGE_OFFSET_MASK
}
