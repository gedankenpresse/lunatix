fn vpn_segments(vaddr: usize) -> [usize; 3] {
    let vpn = [
        (vaddr >> (PBITS + 0 * VPN_BITS)) & VPN_MASK,
        (vaddr >> (PBITS + 1 * VPN_BITS)) & VPN_MASK,
        (vaddr >> (PBITS + 2 * VPN_BITS)) & VPN_MASK,
        // if Sv48, there is a level of page tables more
        // (vaddr >> (12 + 3 * VPN_BITS)) & VPN_BIT_MASK,
    ];
    vpn
}
