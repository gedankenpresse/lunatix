/// Create an allocator that allocates memory from a predefined array laying on the stack.
///
/// # Usage Example
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
#[macro_export]
macro_rules! stack_alloc {
    ($name:ident, $size:literal) => {
        use $crate::bump_allocator::BumpAllocator;
        let mut $name = [0u8; $size];
        #[allow(unused_mut)]
        let mut $name = $crate::bump_allocator::ForwardBumpingAllocator::new(&mut $name);
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
