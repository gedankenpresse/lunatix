/// Create an allocator that allocates memory from a predefined array laying on the stack.
///
/// # Usage Example
///
/// ## Create a generic purpose allocator
/// ```rust
/// # use std::alloc::Layout;
/// # use allocators::{stack_alloc, Allocator, AllocInit};
/// #
/// // create an allocator called `allocator`
/// stack_alloc!(allocator, 1024);
///
/// // use it to perform an allocation
/// let allocation = allocator.allocate(Layout::new::<usize>(), AllocInit::Uninitialized);
/// assert!(allocation.is_ok())
/// ```
///
/// ## Create a specific allocator type
/// ```rust
/// # use std::alloc::Layout;
/// # use allocators::{stack_alloc, Allocator, AllocInit};
/// use allocators::bump_allocator::{ForwardBumpingAllocator, BumpAllocator};
///
/// // create an allocator called `allocator`
/// stack_alloc!(allocator, 1024, ForwardBumpingAllocator);
///
/// // use it to perform an allocation
/// let allocation = allocator.allocate(Layout::new::<usize>(), AllocInit::Uninitialized);
/// assert!(allocation.is_ok())
/// ```
#[macro_export]
macro_rules! stack_alloc {
    ($name:ident, $size:literal, $t:ty) => {
        let mut $name = [0u8; $size];
        let $name = <$t>::new(&mut $name);
    };
    ($name:ident, $size:literal) => {
        use $crate::bump_allocator::BumpAllocator;
        stack_alloc!(
            $name,
            $size,
            $crate::bump_allocator::ForwardBumpingAllocator
        );
    };
}

#[cfg(test)]
mod test {
    extern crate std;

    #[test]
    fn test_stack_alloc_creation() {
        stack_alloc!(_allocator, 2048);
    }
}
