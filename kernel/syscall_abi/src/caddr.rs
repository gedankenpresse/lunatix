use core::fmt;
use core::fmt::{Debug, Display, Formatter, Write};

/// A type for addressing capabilities inside a CSpace
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Copy, Clone)]
#[repr(transparent)]
pub struct CAddr(usize);

impl CAddr {
    #[inline(always)]
    pub const fn new(value: usize) -> Self {
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
    /// let caddr = CAddr::new(0b0001_0011);
    /// let (first, remainder) = caddr.take_bits(4);
    ///
    /// assert_eq!(first, 0b0011);
    /// assert_eq!(remainder.raw(), 0b0001);
    /// ```
    pub const fn take_bits(self, nbits: usize) -> (usize, CAddr) {
        let mask = 2usize.pow(nbits as u32) - 1;
        let taken = self.0 & mask;
        let remainder = self.0 >> nbits;
        (taken, CAddr::new(remainder))
    }
}

impl From<usize> for CAddr {
    #[inline(always)]
    fn from(value: usize) -> Self {
        CAddr::new(value)
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
