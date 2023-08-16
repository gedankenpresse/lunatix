/// The public API which all capabilities must implement
///
/// `U` is recommended to be a union type which bundles specific capabilities together and which a trait implementation
/// can choose freely.
pub trait CapabilityIface<U> {
    type InitArgs;

    /// Initialize a capability of type `self` into the target location.
    ///
    /// It should be guaranteed by the caller that `target` is safe to overwrite.
    fn init(&self, target: &mut U, args: Self::InitArgs);

    /// Copy the capability into a destination location.
    ///
    /// It should be guaranteed by the caller that `dst` is safe to overwrite and that the implementing type of this
    /// trait matches the slot type of `src`.
    fn copy(&self, src: &U, dst: &mut U);

    /// Destroy the capability located at `target`.
    fn destroy(&self, target: &U);
}
