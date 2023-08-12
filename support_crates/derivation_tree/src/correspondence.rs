use core::mem::MaybeUninit;

pub trait Correspondence {
    /// Whether `self` corresponds to the same value as `other`.
    ///
    /// Basically, two derivation tree nodes that are copies of one another should return true because they correspond
    /// to the same capability but two independent nodes (or derivations) should return false.
    fn corresponds_to(&self, other: &Self) -> bool;
}

pub trait CapabilityOps: Sized + Correspondence {
    fn cap_copy(source: &Self, dest: &mut MaybeUninit<Self>);

    fn destroy(&self);
}
