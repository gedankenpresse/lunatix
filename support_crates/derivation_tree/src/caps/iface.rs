use crate::{AsStaticMut, AsStaticRef};

/// The public API which all capabilities must implement
///
/// `U` is recommended to be a union type which bundles specific capabilities together and which a trait implementation
/// can choose freely.
pub trait CapabilityIface<U> {
    /// The arguments which this implementation expects to be passed to [`init()`](Self::init).
    type InitArgs;

    /// Initialize a capability of type `self` into the target location.
    ///
    /// It should be guaranteed by the caller that `target` is safe to overwrite.
    fn init(&self, target: &mut impl AsStaticMut<U>, args: Self::InitArgs);

    /// Copy the capability into a destination location.
    ///
    /// It should be guaranteed by the caller that `dst` is safe to overwrite and that the implementing type of this
    /// trait matches the slot type of `src`.
    fn copy(&self, src: &impl AsStaticRef<U>, dst: &mut impl AsStaticMut<U>);

    /// Destroy the capability located at `target`.
    fn destroy(&self, target: &mut U);
}

/// A trait for easily obtaining the matching *CapabilityIface* for a capability type.
pub trait GetCapIface: Sized {
    /// The type that implements [`CapabilityIface`](CapabilityIface) for `Self`.
    type IfaceImpl: CapabilityIface<Self>;

    /// Get an instance of the type that implements [`CapabilityIface`](CapabilityIface) for `self`.
    fn get_capability_iface(&self) -> Self::IfaceImpl;
}
