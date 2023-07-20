use crate::traits::MutGlobalAlloc;
use core::alloc::Layout;
use core::marker::PhantomData;

/// A free and unallocated block of memory that points to the next free and unallocated block.
#[derive(Copy, Clone, Debug)]
struct FreeBlock {
    next: Option<*mut FreeBlock>,
}

/// An arena allocator implementation
///
/// This allocator is able to reserve (and thus allocate) same size blocks from a continuous slice of memory.
/// These blocks are defined by the `Content` type parameter.
#[derive(Debug)]
pub struct Arena<'a, Content> {
    /// Pointer to the start of the backing memory
    start_ptr: *mut Content,
    /// Number of available to allocate from
    items: usize,
    /// First free, unallocated block in the backing memory
    head: Option<*mut FreeBlock>,
    /// Lifetime hack
    _phantom_data: PhantomData<&'a [Content]>,
}

/// Assert that the given memory address is aligned to `align` bytes
fn is_aligned_to(ptr: *const u8, align: usize) -> bool {
    if !align.is_power_of_two() {
        panic!("is_aligned_to: align is not a power-of-two");
    }

    ptr as usize % align == 0
}

unsafe impl<'a, Content> MutGlobalAlloc for Arena<'a, Content> {
    unsafe fn alloc(&mut self, layout: Layout) -> *mut u8 {
        assert!(layout.align() <= core::mem::align_of::<Content>());
        assert!(layout.size() % core::mem::size_of::<Content>() == 0);
        let blocks = layout.size() / core::mem::size_of::<Content>();
        if blocks == 1 {
            self.alloc_one_impl().cast()
        } else {
            self.alloc_many_impl(blocks).cast()
        }
    }

    unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        assert!(layout.align() <= core::mem::align_of::<Content>());
        assert!(layout.size() % core::mem::size_of::<Content>() == 0);
        let blocks = layout.size() / core::mem::size_of::<Content>();
        if blocks == 1 {
            self.free_one_impl(ptr.cast())
        } else {
            self.free_many_impl(ptr.cast(), blocks)
        }
    }
}

pub unsafe trait ArenaAlloc {
    type Content;

    unsafe fn alloc_one(&mut self) -> *mut Self::Content;
    unsafe fn alloc_many(&mut self, count: usize) -> *mut Self::Content;
}

unsafe impl<'a, Content> ArenaAlloc for Arena<'a, Content> {
    type Content = Content;

    unsafe fn alloc_one(&mut self) -> *mut Self::Content {
        let size = core::mem::size_of::<Content>();
        let align = core::mem::align_of::<Content>();
        let layout = Layout::from_size_align(size, align).unwrap();
        let ptr = self.alloc(layout);
        ptr.cast()
    }

    unsafe fn alloc_many(&mut self, count: usize) -> *mut Self::Content {
        let size = core::mem::size_of::<Content>() * count;
        let align = core::mem::align_of::<Content>();
        let layout = Layout::from_size_align(size, align).unwrap();
        let ptr = self.alloc(layout);
        ptr.cast()
    }
}

impl<'a, Content> Arena<'a, Content> {
    /// Create a new arena allocator from the given slice of memory.
    pub fn new(slice: &'a mut [Content]) -> Self {
        unsafe {
            let mut mem = Self::from_slice(slice);
            mem.init_freelist();
            mem
        }
    }

    /// Initialize a new Arena Allocator from a continuous slice of memory
    fn from_slice(slice: &'a mut [Content]) -> Self {
        let raw = slice.as_mut_ptr();
        let items = slice.len();
        unsafe { Self::from_contigious_blocks(raw, items) }
    }

    /// Create a new Arena Allocator from a given memory area.
    ///
    /// # Safety
    /// `ptr` and `items` need to describe a continuous slice of memory that is unused with lifetime `'a`.
    pub unsafe fn from_contigious_blocks(ptr: *mut Content, items: usize) -> Self {
        assert!(is_aligned_to(
            ptr as *const u8,
            core::mem::align_of::<Content>()
        ));
        assert!(core::mem::size_of::<Content>() >= core::mem::size_of::<FreeBlock>());
        Self {
            start_ptr: ptr,
            items,
            head: None,
            _phantom_data: PhantomData::default(),
        }
    }

    /// Initialize the internal *free-list* to mark the whole memory area as unused.
    ///
    /// Effectively this can be used to reset the allocator state.
    ///
    /// # Safety
    /// Should only ever be called if no objects are allocated from the backing memory.
    pub unsafe fn init_freelist(&mut self) {
        for i in 0..self.items {
            let block = self.start_ptr.add(i).cast::<FreeBlock>();
            if i == self.items - 1 {
                *block = FreeBlock { next: None };
            } else {
                *block = FreeBlock {
                    next: Some(block.cast::<Content>().add(1).cast::<FreeBlock>()),
                };
            }
        }
        self.head = Some(self.start_ptr.cast::<FreeBlock>());
    }

    /// Allocate one block from the arena and return a pointer to it.
    ///
    /// This is the raw pointer variant of [`alloc_one()`](Arena::alloc_one) which should be preferred over this function
    /// when possible.
    unsafe fn alloc_one_impl(&mut self) -> *mut Content {
        match self.head {
            Some(block_ptr) => {
                self.head = unsafe { (*block_ptr).next };
                unsafe {
                    *block_ptr = FreeBlock { next: None };
                }
                block_ptr.cast()
            }
            None => core::ptr::null_mut(),
        }
    }

    /// Allocate `items` number of blocks from the arena and return a reference to the first one.
    /// The allocation logic guarantees these objects to be continuously placed.
    unsafe fn alloc_many_impl(&mut self, items: usize) -> *mut Content {
        unsafe {
            let mut count = 1;
            let mut cur_head: *mut FreeBlock = match self.head {
                Some(b) => b,
                None => return core::ptr::null_mut(),
            };
            let mut cur = cur_head;
            while count < items {
                let next = match (*cur).next {
                    Some(block) => block,
                    None => return core::ptr::null_mut(),
                };
                if next == cur.cast::<Content>().offset(1).cast::<FreeBlock>() {
                    cur = next;
                    count += 1;
                } else {
                    cur_head = next;
                    cur = next;
                    count = 1;
                }
            }
            self.head = (*cur).next;
            return cur_head.cast();
        }
    }

    /// Free the given memory allocation
    ///
    /// # Safety
    /// The memory must no longer be used and must have been allocated from this allocator.
    unsafe fn free_one_impl(&mut self, ptr: *mut Content) {
        assert!(is_aligned_to(
            ptr as *const u8,
            core::mem::align_of::<Content>()
        ));
        assert!(ptr >= self.start_ptr);
        assert!(ptr < self.start_ptr.offset(self.items as isize));
        let block_ptr = ptr.cast::<FreeBlock>();
        (*block_ptr).next = self.head;
        self.head = Some(block_ptr);
    }

    /// Free multiple allocations starting at `ptr` and containing `items` blocks.
    ///
    /// # Safety
    /// All blocks must no longer be used and must have been allocated from this allocator.
    unsafe fn free_many_impl(&mut self, ptr: *mut Content, items: usize) {
        assert!(is_aligned_to(
            ptr as *const u8,
            core::mem::align_of::<Content>()
        ));
        assert!(ptr >= self.start_ptr);
        assert!(ptr.offset(items as isize) <= self.start_ptr.offset(self.items as isize));
        for i in 0..items {
            self.free_one_impl(ptr.offset(i as isize));
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use std::vec::Vec;

    use super::ArenaAlloc;
    use crate::traits::tests as alloc_tests;

    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    struct Point {
        x: usize,
        y: usize,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    #[repr(align(4096))]
    struct Page([u8; 4096]);

    #[test]
    fn can_create_memory() {
        const ITEMS: usize = 20;
        let mut points = [Point { x: 0, y: 0 }; ITEMS];
        let raw = points.as_mut_ptr();
        unsafe {
            let mut mem = super::Arena::from_contigious_blocks(raw, ITEMS);
            mem.init_freelist();
        }
    }

    #[test]
    fn can_alloc_single() {
        const ITEMS: usize = 1;
        let mut points = [Point { x: 0, y: 0 }; ITEMS];
        let mut mem = super::Arena::new(&mut points);

        alloc_tests::can_alloc_free_single::<Point>(&mut mem);
    }

    #[test]
    fn can_alloc_memory() {
        const ITEMS: usize = 20;
        let mut points = [Point { x: 0, y: 0 }; ITEMS];
        let mut mem = super::Arena::new(&mut points);

        alloc_tests::can_alloc_free_count::<Point>(&mut mem, ITEMS);
    }

    #[test]
    fn can_alloc_page() {
        use std::vec::Vec;
        const ITEMS: usize = 200;
        let mut pages = Vec::with_capacity(ITEMS);
        for _ in 0..ITEMS {
            let page: Page = Page([0; 4096]);
            pages.push(page);
        }
        assert!(pages.len() == pages.capacity());
        let len = pages.len();
        let mut mem = super::Arena::new(&mut pages[0..len]);
        alloc_tests::can_alloc_free_count::<Page>(&mut mem, ITEMS);
    }

    #[test]
    fn can_alloc_pages() {
        use std::vec::Vec;
        const ITEMS: usize = 200;
        let mut pages = Vec::with_capacity(ITEMS);
        for _ in 0..ITEMS {
            let page: Page = Page([0; 4096]);
            pages.push(page);
        }
        assert!(pages.len() == pages.capacity());
        let len = pages.len();
        let mut mem = super::Arena::new(&mut pages[0..len]);
        assert!(unsafe { !mem.alloc_many(20).is_null() });
    }

    #[test]
    fn allocs_dont_alias() {
        const ITEMS: usize = 20;
        let mut points = [Point { x: 0, y: 0 }; ITEMS];
        let mut mem = super::Arena::new(&mut points);

        let mut items = Vec::new();
        for i in 0..ITEMS {
            items.push(Point { x: i, y: i });
        }
        alloc_tests::allocs_dont_alias(&mut mem, &items);
    }
}
