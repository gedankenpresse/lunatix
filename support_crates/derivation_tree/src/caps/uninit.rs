use crate::correspondence::Correspondence;

/// A capability that does not actually do anything but marks an uninitialized memory region that is safe to overwrite
/// with other capabilities.
pub struct Uninit;

impl Correspondence for Uninit {
    fn corresponds_to(&self, _other: &Self) -> bool {
        false
    }
}
