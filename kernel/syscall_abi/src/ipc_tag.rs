use core::fmt::{Debug, Formatter};

/// An IpcTag stores metadata for an IPC `call` or `send` operation.
///
/// It stores the fields `label`, `ncaps` and `nparams` tightly packed into one usize (in that order).
#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct IpcTag(usize);

pub const NPARAM_BITS: usize = 3;
pub const NCAP_BITS: usize = 3;
pub const LABEL_BITS: usize = 64 - NPARAM_BITS - NCAP_BITS;

const NPARAM_MASK: usize = (1 << NPARAM_BITS) - 1;
const NCAP_MASK: usize = (1 << NCAP_BITS) - 1;
const LABEL_MASK: usize = (1 << LABEL_BITS) - 1;

impl IpcTag {
    /// Create a new IpcTag from its raw representation
    #[inline(always)]
    pub const fn from_raw(raw: usize) -> Self {
        Self(raw)
    }

    /// Get the raw representation of this tag
    #[inline(always)]
    pub const fn as_raw(&self) -> usize {
        self.0
    }

    #[inline(always)]
    pub fn from_parts(label: usize, ncaps: u8, nparams: u8) -> Self {
        assert!(
            label <= LABEL_MASK,
            "cannot use more than {} bits for the label value",
            LABEL_BITS
        );
        assert!(
            ncaps <= NCAP_MASK as u8,
            "cannot use more than {} bits for the ncap value",
            NCAP_BITS
        );
        assert!(
            nparams <= NPARAM_MASK as u8,
            "cannot use more than {} bits for the nparam value",
            NPARAM_MASK
        );
        Self(
            label << NPARAM_BITS << NCAP_BITS
                | (ncaps as usize) << (NPARAM_BITS)
                | (nparams as usize),
        )
    }

    #[inline(always)]
    pub fn nparams(&self) -> u8 {
        (self.0 & NPARAM_MASK) as u8
    }

    #[inline(always)]
    pub fn ncaps(&self) -> u8 {
        ((self.0 >> NPARAM_BITS) & NCAP_MASK) as u8
    }

    #[inline(always)]
    pub fn label(&self) -> usize {
        (self.0 >> NCAP_BITS >> NPARAM_BITS) & LABEL_MASK
    }
}

impl From<usize> for IpcTag {
    #[inline(always)]
    fn from(value: usize) -> Self {
        Self::from_raw(value)
    }
}

impl From<IpcTag> for usize {
    #[inline(always)]
    fn from(value: IpcTag) -> Self {
        value.as_raw()
    }
}

impl Debug for IpcTag {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let is_alternate = f.alternate();
        let mut s = f.debug_struct("IpcTag");
        s.field("nparams", &self.nparams())
            .field("ncaps", &self.ncaps())
            .field("label", &self.label());
        if is_alternate {
            s.field("raw", &self.as_raw());
        }
        s.finish()
    }
}
