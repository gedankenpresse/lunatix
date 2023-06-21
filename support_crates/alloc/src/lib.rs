#![no_std]

use core::marker::PhantomData;
use core::mem::MaybeUninit;

#[cfg(feature = "std")]
extern crate std;

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

    /// Allocate one block from the arena and return it
    pub fn alloc_one<'b>(&'b mut self) -> Option<&'a mut MaybeUninit<Content>> {
        let raw = match self.alloc_one_raw() {
            Some(raw) => raw,
            None => return None,
        };

        unsafe { Some(&mut (*raw)) }
    }

    /// Allocate one block from the arena and return a pointer to it.
    ///
    /// This is the raw pointer variant of [`alloc_one()`](Arena::alloc_one) which should be preferred over this function
    /// when possible.
    pub fn alloc_one_raw(&mut self) -> Option<*mut MaybeUninit<Content>> {
        match self.head {
            Some(block_ptr) => {
                self.head = unsafe { (*block_ptr).next };
                unsafe {
                    *block_ptr = FreeBlock { next: None };
                }
                Some(block_ptr.cast::<MaybeUninit<Content>>())
            }
            None => None,
        }
    }

    /// Allocate `items` number of blocks from the arena and return a reference to the allocated slice.
    /// If the allocation succeeds, the slice guarantees these objects to be continuously placed.
    pub fn alloc_many<'b>(&'b mut self, items: usize) -> Option<&'a mut [MaybeUninit<Content>]> {
        let raw = match self.alloc_many_raw(items) {
            Some(b) => b,
            None => return None,
        };

        return Some(unsafe { core::slice::from_raw_parts_mut(raw, items) });
    }

    /// Allocate `items` number of blocks from the arena and return a reference to the first one.
    /// The allocation logic guarantees these objects to be continuously placed.
    pub fn alloc_many_raw(&mut self, items: usize) -> Option<*mut MaybeUninit<Content>> {
        unsafe {
            let mut count = 1;
            let mut cur_head: *mut FreeBlock = match self.head {
                Some(b) => b,
                None => return None,
            };
            let mut cur = cur_head;
            while count < items {
                let next = match (*cur).next {
                    Some(block) => block,
                    None => return None,
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
            Some(cur_head.cast::<MaybeUninit<Content>>())
        }
    }

    /// Free the given memory allocation
    ///
    /// # Safety
    /// The memory must no longer be used and must have been allocated from this allocator.
    pub unsafe fn free_one(&mut self, ptr: *mut Content) {
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
    pub unsafe fn free_many(&mut self, ptr: *mut Content, items: usize) {
        assert!(is_aligned_to(
            ptr as *const u8,
            core::mem::align_of::<Content>()
        ));
        assert!(ptr >= self.start_ptr);
        assert!(ptr.offset(items as isize) <= self.start_ptr.offset(self.items as isize));
        for i in 0..items {
            self.free_one(ptr.offset(i as isize));
        }
    }
}

#[cfg(test)]
mod tests {

    #[cfg(test)]
    extern crate std;

    #[derive(Copy, Clone)]
    struct Point {
        x: usize,
        y: usize,
    }

    type Page = [u8; 4096];

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
    fn can_alloc_memory() {
        const ITEMS: usize = 20;
        let mut points = [Point { x: 0, y: 0 }; ITEMS];
        let mut mem = super::Arena::new(&mut points);

        for i in 0..ITEMS {
            assert!(mem.alloc_one().is_some(), "failed to alloc {i}");
        }
        assert!(mem.alloc_one().is_none());
    }

    #[test]
    fn can_alloc_page() {
        use std::vec::Vec;
        const ITEMS: usize = 200;
        let mut pages = Vec::with_capacity(ITEMS);
        for _ in 0..ITEMS {
            let page: Page = [0; 4096];
            pages.push(page);
        }
        assert!(pages.len() == pages.capacity());
        let len = pages.len();
        let mut mem = super::Arena::new(&mut pages[0..len]);
        for i in 0..ITEMS {
            assert!(mem.alloc_one().is_some(), "could not alloc {i}");
        }
        assert!(mem.alloc_one().is_none());
    }

    #[test]
    fn can_alloc_pages() {
        use std::vec::Vec;
        const ITEMS: usize = 200;
        let mut pages = Vec::with_capacity(ITEMS);
        for _ in 0..ITEMS {
            let page: Page = [0; 4096];
            pages.push(page);
        }
        assert!(pages.len() == pages.capacity());
        let len = pages.len();
        let mut mem = super::Arena::new(&mut pages[0..len]);
        assert!(mem.alloc_many_raw(20).is_some());
    }

    #[test]
    fn allocs_dont_alias() {
        const ITEMS: usize = 20;
        let mut points = [Point { x: 0, y: 0 }; ITEMS];
        let mut mem = super::Arena::new(&mut points);

        let mut alloc_points = [None; ITEMS];
        for i in 0..ITEMS {
            alloc_points[i] = Some(unsafe {
                let point_raw = mem.alloc_one_raw().unwrap();
                let point = &mut *point_raw;
                *point = Point { x: i, y: i };
                point_raw
            });
        }
        for i in 0..ITEMS {
            assert!(alloc_points[i].is_some());

            unsafe {
                assert_eq!((*alloc_points[i].unwrap()).x, i);
                assert_eq!((*alloc_points[i].unwrap()).y, i);
            }
        }
    }

    #[test]
    fn can_free_one() {
        const ITEMS: usize = 20;
        let mut points = [Point { x: 0, y: 0 }; ITEMS];
        let mut mem = super::Arena::new(&mut points);

        for _ in 0..ITEMS * 2 {
            let ptr = mem.alloc_one().unwrap();
            unsafe {
                mem.free_one(ptr);
            }
        }
    }

    #[test]
    fn can_use_allocs() {
        const ITEMS: usize = 20;
        let mut points = [Point { x: 0, y: 0 }; ITEMS];
        let mut mem = super::Arena::new(&mut points);
        let block = mem.alloc_one().unwrap();
        *block = Point { x: 1, y: 1 };
        drop(points);
    }

    #[test]
    fn can_alloc_memory_by_ones() {
        const ITEMS: usize = 20;
        let mut points = [Point { x: 0, y: 0 }; ITEMS];
        let mut mem = super::Arena::new(&mut points);

        for i in 0..ITEMS {
            assert!(mem.alloc_many(1).is_some(), "failed to alloc {i}");
        }
        assert!(mem.alloc_one().is_none());
    }

    #[test]
    fn can_alloc_many() {
        const BLOCKS: usize = 5;
        const SIZE: usize = 2;
        const ITEMS: usize = BLOCKS * SIZE;
        let mut points = [Point { x: 0, y: 0 }; ITEMS];
        let mut mem = super::Arena::new(&mut points);
        for _ in 0..BLOCKS {
            let _ = mem.alloc_many(2).unwrap();
        }
        assert!(mem.alloc_one().is_none());
    }

    #[test]
    fn alloc_many_dont_alias() {
        const BLOCKS: usize = 5;
        const SIZE: usize = 2;
        const ITEMS: usize = BLOCKS * SIZE;
        let mut points = [Point { x: 0, y: 0 }; ITEMS];
        let mut mem = super::Arena::new(&mut points);

        let mut alloced: [Option<&mut [Point]>; BLOCKS] = [None, None, None, None, None];
        for i in 0..BLOCKS {
            alloced[i] = mem.alloc_many(SIZE);
            assert!(alloced[i].is_some());
            assert!(alloced[i].as_ref().unwrap().len() == SIZE);
            alloced[i].as_deref_mut().unwrap()[0] = Point { x: i, y: i };
            alloced[i].as_deref_mut().unwrap()[1] = Point { x: i, y: i };
        }
        assert!(mem.alloc_one().is_none());

        for i in 0..BLOCKS {
            assert!(alloced[i].as_ref().unwrap()[0].x == i);
            assert!(alloced[i].as_ref().unwrap()[0].y == i);
            assert!(alloced[i].as_ref().unwrap()[1].x == i);
            assert!(alloced[i].as_ref().unwrap()[1].y == i);
        }
    }

    /*
    // This Test *shouldn't* compile
    #[test]
    fn cant_leak_allocs() {
        const ITEMS: usize = 20;
        let mut points = [Point { x: 0, y: 0}; ITEMS];
        let mut mem = super::Memory::new(&mut points);
        let block = mem.alloc_one().unwrap();
        *block = Point { x: 1, y: 1};
        drop(points);
        *block = Point { x: 1, y: 1};
    }
    */

    #[test]
    fn can_free_many() {
        const BLOCKS: usize = 5;
        const SIZE: usize = 2;
        const ITEMS: usize = BLOCKS * SIZE;
        let mut points = [Point { x: 0, y: 0 }; ITEMS];
        let mut mem = super::Arena::new(&mut points);

        let mut alloced: [Option<*mut Point>; BLOCKS] = [None, None, None, None, None];
        for i in 0..BLOCKS {
            alloced[i] = mem.alloc_many_raw(SIZE);
            assert!(alloced[i].is_some());
        }
        assert!(mem.alloc_one().is_none());

        for i in 0..BLOCKS {
            unsafe {
                mem.free_many(alloced[i].unwrap(), SIZE);
            }
        }

        for i in 0..ITEMS {
            assert!(mem.alloc_one().is_some(), "failed to alloc {i}");
        }
        assert!(mem.alloc_one().is_none());
    }
}
