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
///   let addr = CAddr::new(2, 3);
///   ```
///
/// - **/2** (cspace cap):
///
///   Addressing the CSpace works exactly the same as addressing any other capabilities.
///
///   ```rust
///   # use syscall_abi::CAddr;
///   let addr = CAddr::new(2, 3);
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
///   So in this concrete example, the address is constructed using a builder because we want to address the nested
///   hierarchy.
///   The */2* CSpace is selected with `.part(2, 3)` for the same reason as the example above.
///   Since that CSpace has capacity for two capabilities, it requires 1 bit to address them.
///   Therefore, the memory capability at slot 0 is selected via `.part(0, 1)`.
///
///   ```rust
///   # use syscall_abi::CAddr;
///   let addr = CAddr::builder().part(2, 3).part(0, 1).finish();
///   ```
///
/// # Address Encoding
///
/// The kernel needs to know when one part of a CAddr is over and whether there is another part to it or not.
/// This is necessary so that it can distinguish between an address that addresses a CSpace vs content in that CSpace.
///
/// To do this, CAddrs are encoded using a continuation bit between each part.
/// If that bit is `0`, the address has no further parts. Otherwise it is `1`.
///
/// This means that for example an address with the parts `0b0001 / 0b011` is encoded like this:
/// ```txt
///   cont. bit
///       │
/// 0b011_1_0001
///   │       │
///part 2   part 1
/// ```
///
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
#[repr(transparent)]
pub struct CAddr(usize);

impl CAddr {
    /// Create a new CAddr with a single part that points to `value`.
    ///
    /// To construct more complex, hierarchical CAddrs use the builder via [`CAddr::builder()`](Self::builder).
    #[inline(always)]
    pub const fn new(value: usize, nbits: usize) -> Self {
        let part_mask = 2usize.pow(nbits as u32) - 1;
        assert!(value <= part_mask, "value uses more than nbits");
        Self(value)
    }

    /// Create a builder for constructing complex CAddrs with multiple parts.
    ///
    /// ```rust
    /// # use syscall_abi::CAddr;
    /// let addr = CAddr::builder().part(0b0001, 4).part(0b011, 3).finish();
    /// assert_eq!(addr.raw(), 0b011_1_0001);
    /// ```
    pub fn builder() -> CAddrBuilder<32> {
        CAddrBuilder { parts: [None; 32] }
    }

    #[inline(always)]
    pub const fn from_raw(value: usize) -> Self {
        Self(value)
    }

    #[inline(always)]
    pub const fn raw(&self) -> usize {
        self.0
    }

    /// Take the first `nbits` of this address and return the part that those bits encode.
    /// The second value is the remaining parts of the CAddr depending on the value of the continuation bit.
    ///
    /// # Example
    /// ```rust
    /// # use syscall_abi::CAddr;
    /// let caddr = CAddr::from_raw(0b011_1_0001);
    /// let (first, remainder) = caddr.take_bits(4);
    ///
    /// assert_eq!(first, 0b0001);
    /// assert_eq!(remainder.unwrap().raw(), 0b_011);
    /// ```
    pub const fn take_bits(self, nbits: usize) -> (usize, Option<CAddr>) {
        let part_mask = 2usize.pow(nbits as u32) - 1;
        let part = self.0 & part_mask;
        let remainder = self.0 >> nbits;

        if remainder & 1 == 0 {
            (part, None)
        } else {
            (part, Some(CAddr::from_raw(remainder >> 1)))
        }
    }

    /// Add a part to this CAddr that uses `nbits` to store
    fn add_part(self, value: usize, nbits: usize) -> Self {
        assert!(value <= 2usize.pow(nbits as u32) - 1);

        // add continuation bit
        let mut previous = (self.0 << 1) | 1;

        // shift previous content to make space for the new one
        previous = previous << nbits;

        Self(previous | value)
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

impl fmt::Binary for CAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        fmt::Binary::fmt(&self.0, f)
    }
}

/// A helper struct to construct a [`CAddr`] from multiple parts
pub struct CAddrBuilder<const MAX_PARTS: usize> {
    parts: [Option<(usize, usize)>; MAX_PARTS],
}

impl<const MAX_PARTS: usize> CAddrBuilder<MAX_PARTS> {
    fn get_free_slot(&mut self) -> &mut Option<(usize, usize)> {
        self.parts
            .iter_mut()
            .find(|slot| slot.is_none())
            .expect("CAddrBuilder does not have a free slot left")
    }

    /// Add a part to the CAddr with the given value that uses `nbits` to store itself
    pub fn part(mut self, value: usize, nbits: usize) -> Self {
        assert!(value <= 2usize.pow(nbits as u32) - 1);

        let slot = self.get_free_slot();
        *slot = Some((value, nbits));
        self
    }

    /// Finish the CAddr construction
    pub fn finish(self) -> CAddr {
        let mut parts = self.parts.iter().filter_map(|&part| part).rev();

        // assert that the constructed CAddr is valid
        let n_part_bits = parts
            .clone()
            .fold(0usize, |acc, (_, i_nbits)| acc + i_nbits);
        let n_cont_bits = parts.clone().count() - 1;
        assert!(n_part_bits + n_cont_bits <= 64, "The total number of parts uses more than 64 bits and therefore cannot be stored in a CAddr");

        // construct the resulting address
        let (init_value, init_nbits) = parts.next().expect(
            "A CAddr must contain at least one part but none have been added to the builder",
        );
        parts.fold(
            CAddr::new(init_value, init_nbits),
            |acc, (i_value, i_nbits)| acc.add_part(i_value, i_nbits),
        )
    }
}
