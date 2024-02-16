/// Type alias for virtual addresses.
///
/// This is used by functions that explicitly interpret addresses as virtual ones.
pub type VAddr = u64;

const PAGE_OFFSET_BITS: u64 = 12;
const PAGE_OFFSET_MASK: u64 = (1 << PAGE_OFFSET_BITS) - 1;

const VPN_SEGMENT_BITS: u64 = 9;
const VPN_SEGMENT_MASK: u64 = (1 << VPN_SEGMENT_BITS) - 1;
const VPN0_MASK: u64 = VPN_SEGMENT_MASK << (PAGE_OFFSET_BITS + 0 * VPN_SEGMENT_BITS);
const VPN1_MASK: u64 = VPN_SEGMENT_MASK << (PAGE_OFFSET_BITS + 1 * VPN_SEGMENT_BITS);
const VPN2_MASK: u64 = VPN_SEGMENT_MASK << (PAGE_OFFSET_BITS + 2 * VPN_SEGMENT_BITS);
const VPN_MASK: u64 = VPN0_MASK | VPN1_MASK | VPN2_MASK;

/// Get the VPN (virtual page number) segments from a virtual address
#[inline]
pub fn vaddr_vpn_segments(vaddr: VAddr) -> [u64; 3] {
    [
        (vaddr & VPN0_MASK) >> (PAGE_OFFSET_BITS + 0 * VPN_SEGMENT_BITS),
        (vaddr & VPN1_MASK) >> (PAGE_OFFSET_BITS + 1 * VPN_SEGMENT_BITS),
        (vaddr & VPN2_MASK) >> (PAGE_OFFSET_BITS + 2 * VPN_SEGMENT_BITS),
    ]
}

/// Get the virtual page number encoded in a physical address
#[inline]
pub fn vaddr_vpn(paddr: VAddr) -> u64 {
    (paddr & VPN_MASK) >> PAGE_OFFSET_BITS
}

/// Construct a physical address corresponding to the given virtual page
#[inline]
pub fn vaddr_from_vpn(vpn: u64) -> VAddr {
    (vpn << PAGE_OFFSET_BITS) & VPN_MASK
}

/// Get the page offset from a virtual address
#[inline]
pub fn vaddr_page_offset(vaddr: VAddr) -> u64 {
    vaddr & PAGE_OFFSET_MASK
}
