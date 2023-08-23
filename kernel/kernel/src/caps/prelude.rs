pub type CapCounted<T> = derivation_tree::CapCounted<'static, 'static, T>;
pub type KernelAlloc = allocators::bump_allocator::ForwardBumpingAllocator<'static>;
