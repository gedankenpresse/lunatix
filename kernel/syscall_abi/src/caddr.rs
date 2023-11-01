use core::fmt;
use core::fmt::{Debug, Formatter};

/// A type for addressing capabilities inside a CSpace
///
/// # Address Hierarchy
///
/// CAddrs are used to address capabilities not just in one CSpace but in a hierarchy of CSpaces.
/// Every capability in this hierarchy can be addressed with one single CAddr.
///
/// For example, imagine a task has access to the following capabilities:
/// ```txt
/// tasks root cspace
///  ├ 0: vspace cap
///  ├ 1: memory cap
///  ├ 2: cspace cap
///  │     ├ 0: memory cap
///  │     └ 1: endpoint cap
///  └ 3: endpoint cap
/// ```
///
/// This is how some capabilities in that hierarchy can be addressed:
///
/// - **/0** (vspace cap):
///
///   The root CSpace has 4 slots so we need 3 bits to address a capability in it.
///   So to address the 0th slot, we construct a CAddr with a part-value of `0` but tell it that
///   3 bits should be used to store that 0.
///
///   ```rust
///   # use syscall_abi::CAddr;
///   let addr = CAddr::new().add_part(0, 3);
///   ```
///
/// - **/2** (cspace cap):
///
///   Addressing the CSpace works exactly the same as addressing any other capabilities.
///
///   ```rust
///   # use syscall_abi::CAddr;
///   let addr = CAddr::new().add_part(2, 3);
///   ```
///
/// - **/2/0** (memory cap):
///
///   To address a capability that is located further down in the hierarchy, multiple parts need to be added to a
///   CAddr.
///   At first, the CSpace that should be used as the parent for the next part needs to be selected in the same way
///   as the previous example.
///   Afterwards, when the capacity of the selected CSpace is known, another part should be added with `nbits` set
///   accordingly.
///
///   So in this concrete example, the */2* CSpace is selected with `.add_part(2, 3)` and since it has capacity for two
///   capabilities, it requires 1 bit to address its capabilities.
///   Therefore, the memory capability at slot 0 is selected via `.add_part(0, 1)`.
///
///   ```rust
///   # use syscall_abi::CAddr;
///   let addr = CAddr::new().add_part(2, 3).add_part(0, 1);
///   ```
///
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Copy, Clone)]
#[repr(transparent)]
pub struct CAddr(usize);

impl CAddr {
    /// Create a new empty CAddr that points to nothing.
    ///
    /// To add parts to the address, use [`add_part`](Self::add_part).
    #[inline(always)]
    pub const fn new() -> Self {
        Self(usize::MAX)
    }

    #[inline(always)]
    pub const fn from_raw(value: usize) -> Self {
        Self(value)
    }

    #[inline(always)]
    pub const fn raw(&self) -> usize {
        self.0
    }

    /// Take the first `nbits` of this address and return the pair of those bits with the remaining bits as a new CAddr
    ///
    /// # Example
    /// ```rust
    /// # use syscall_abi::CAddr;
    /// let caddr = CAddr::from_raw(0b0001_0011);
    /// let (first, remainder) = caddr.take_bits(4);
    ///
    /// assert_eq!(first, 0b0011);
    /// assert_eq!(remainder.raw(), 0b0001);
    /// ```
    pub const fn take_bits(self, nbits: usize) -> (usize, CAddr) {
        let mask = 2usize.pow(nbits as u32) - 1;
        let taken = self.0 & mask;
        let remainder = self.0 >> nbits;
        (taken, CAddr::from_raw(remainder))
        // TODO Add 0b1111 to high bits
    }

    /// Add a part to the address that uses `nbits` for its value.
    ///
    /// ```rust
    /// # use syscall_abi::CAddr;
    /// let addr = CAddr::new().add_part(0b0001, 4).add_part(0b011, 3);
    /// assert_eq!(addr.raw(), 0b111111111111111111111111111111111111111111111111111111111_0001_011);
    /// ```
    pub const fn add_part(self, value: usize, nbits: usize) -> Self {
        // TODO assert that the left-most nbits of self are all 1

        let previous = self.0 << nbits;
        let part_mask = 2usize.pow(nbits as u32) - 1;
        assert!(
            value <= part_mask,
            "value uses more than nbits for its value"
        );
        Self::from_raw(previous | value)
    }
}

impl From<usize> for CAddr {
    #[inline(always)]
    fn from(value: usize) -> Self {
        CAddr::from_raw(value)
    }
}

impl Into<usize> for CAddr {
    #[inline(always)]
    fn into(self) -> usize {
        self.raw()
    }
}

impl fmt::LowerHex for CAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        fmt::LowerHex::fmt(&self.0, f)
    }
}

impl fmt::UpperHex for CAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        fmt::UpperHex::fmt(&self.0, f)
    }
}
