/// An comparison operator to determine if two capabilities correspond to each other.
///
/// When capabilities are copied, they usually refer to the same internal data via an internally managed smart-pointer.
/// To determine if two capability instances are referring to the same thing (for example two memory capabilities
/// use the same backing memory), they are expected to implement this trait.
pub trait Correspondence {
    /// Whether `self` corresponds to the same value as `other`.
    ///
    /// Basically, two derivation tree nodes that are copies of one another should return true because they correspond
    /// to the same capability but two independent nodes (or derivations) should return false.
    fn corresponds_to(&self, other: &Self) -> bool;
}
