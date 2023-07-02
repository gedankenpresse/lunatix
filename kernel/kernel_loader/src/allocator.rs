/// A simple allocator implementation which simply points to free memory.
/// As a consequence, it only supports allocations but not freeing.
#[derive(Debug)]
pub struct BumpAllocator {
    start: *mut u8,
    end: *mut u8,
}

impl BumpAllocator {
    pub unsafe fn new(start: *mut u8, end: *mut u8) -> Self {
        assert!(start <= end);
        Self { start, end }
    }

    /// How much free space (in bytes) remains in the allocators backing memory
    pub fn capacity(&self) -> usize {
        self.end as usize - self.start as usize
    }

    pub fn into_raw(self) -> (*mut u8, *mut u8) {
        let Self { start, end } = self;
        return (start, end);
    }

    /// Allocate a certain number of bytes with a given alignment.
    /// Returns `None` if not enough free space is available.
    pub fn alloc(&mut self, size: usize, alignment: usize) -> Option<*mut u8> {
        assert!(alignment.is_power_of_two());
        assert_ne!(size, 0);
        let aligned_start =
            (self.start as usize).checked_add(self.start.align_offset(alignment))? as *mut u8;
        if aligned_start as usize + size > self.end as usize {
            None
        } else {
            self.start = unsafe { aligned_start.add(size) };

            // zero content, this should actually be done in the elf loader...
            // TODO: don't do this step and initialize kernel correctly
            unsafe {
                let mut start = aligned_start;
                let end = aligned_start.add(size);
                while start < end {
                    *start = 0;
                    start = start.add(1);
                }
            };
            Some(aligned_start)
        }
    }
}
