mod allocator;

pub use allocator::BoundaryTagAllocator;

mod tags;
#[cfg(test)]
mod tests;
