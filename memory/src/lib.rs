#![no_std]

use core::marker::PhantomData;

#[cfg(feature = "std")]
extern crate std;

#[derive(Copy, Clone, Debug)]
struct Block {
    next: Option<*mut Block>,
}

#[derive(Debug)]
pub struct Memory<'a, Content> {
    start_ptr: *mut Content,
    items: usize,
    head: Option<*mut Block>,
    phantom_data: PhantomData<&'a [Content]>,
}

fn is_aligned_to(ptr: *const u8, align: usize) -> bool {
    if !align.is_power_of_two() {
        panic!("is_aligned_to: align is not a power-of-two");
    }

    ptr as usize % align == 0
}


impl<'a, Content> Memory<'a, Content> {
    pub unsafe fn from_slice(slice: &'a mut [Content]) -> Self {
        let raw = slice.as_mut_ptr();
        let items = slice.len();
        unsafe { Self::from_contigious_blocks(raw, items) }
    }

    pub fn new(slice: &'a mut [Content]) -> Self {
        unsafe {
            let mut mem = Self::from_slice(slice);
            mem.init_freelist();
            mem
        }
    }
}

impl<'a, Content> Memory<'a, Content> {
    pub unsafe fn from_contigious_blocks(
        ptr: *mut Content,
        items: usize,
    ) -> Self {
        assert!(is_aligned_to(ptr as *const u8, core::mem::align_of::<Content>()));
        assert!(core::mem::size_of::<Content>() >= core::mem::size_of::<Block>());
        Self {
            start_ptr: ptr,
            items,
            head: None,
            phantom_data: PhantomData::default(),
        }
    } 

    pub unsafe fn init_freelist(&mut self) {
        for i in 0..self.items {
            let block = self.start_ptr.add(i).cast::<Block>();
            if i == self.items - 1 {
                *block = Block { next: None };
            } else {
                *block = Block { next: Some(block.cast::<Content>().add(1).cast::<Block>()) };
            }
        }
        self.head = Some(self.start_ptr.cast::<Block>());
    }

    pub fn alloc_one<'b>(&'b mut self) -> Option<&'a mut Content> {
        let raw = match self.alloc_one_raw() {
            Some(raw) => raw,
            None => return None,
        };

        unsafe { Some(&mut (*raw)) }
    }

    pub fn alloc_one_raw(&mut self) -> Option<*mut Content> {
        match self.head {
            Some(block_ptr) => {
                self.head = unsafe { (*block_ptr).next };
                unsafe { *block_ptr = Block { next: None }; }
                Some(block_ptr.cast::<Content>())
            },
            None => None,
        }
    }

    pub fn alloc_many<'b>(&'b mut self, items: usize) -> Option<&'a mut [Content]> {
        let raw = match self.alloc_many_raw(items) {
            Some(b) => b,
            None => return None,
        };

        return Some(unsafe { core::slice::from_raw_parts_mut(raw, items) });
    }

    pub fn alloc_many_raw(&mut self, items: usize) -> Option<*mut Content> {
        unsafe {
            let mut count = 1;
            let mut cur_head: *mut Block = match self.head {
                Some(b) => b,
                None => return None,
            };
            let mut cur = cur_head;
            while count < items {
                let next = match (*cur).next {
                    Some(block) => block,
                    None => return None,
                };
                if next == cur.cast::<Content>().offset(1).cast::<Block>() {
                    cur = next;
                    count += 1;
                } else {
                    cur_head = next;
                    cur = next;
                    count = 1;
                }
            }
            self.head = (*cur).next;
            return Some(cur_head.cast::<Content>());
        }
    }

    pub unsafe fn free_one(&mut self, ptr: *mut Content) {
        assert!(is_aligned_to(ptr as *const u8, core::mem::align_of::<Content>()));
        assert!(ptr >= self.start_ptr);
        assert!(ptr < self.start_ptr.offset(self.items as isize));
        let block_ptr = ptr.cast::<Block>();
        (*block_ptr).next = self.head;
        self.head = Some(block_ptr);
    }

    pub unsafe fn free_many(&mut self, ptr: *mut Content, items: usize) {
        assert!(is_aligned_to(ptr as *const u8, core::mem::align_of::<Content>()));
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
    struct Point { x: usize, y: usize }


    type Page = [u8; 4096];

    #[test]
    fn can_create_memory() {
        const ITEMS: usize = 20;
        let mut points = [Point { x: 0, y: 0}; ITEMS];
        let raw = points.as_mut_ptr();
        unsafe {
            let mut mem = super::Memory::from_contigious_blocks(raw, ITEMS);
            mem.init_freelist();
        }
    }

    #[test]
    fn can_alloc_memory() {
        const ITEMS: usize = 20;
        let mut points = [Point { x: 0, y: 0}; ITEMS];
        let mut mem = super::Memory::new(&mut points);

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
        let mut mem = super::Memory::new(&mut pages[0..len]);
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
        let mut mem = super::Memory::new(&mut pages[0..len]);
        assert!(mem.alloc_many_raw(20).is_some());
    }

    #[test]
    fn allocs_dont_alias() {
        const ITEMS: usize = 20;
        let mut points = [Point { x: 0, y: 0}; ITEMS];
        let mut mem = super::Memory::new(&mut points);

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
        let mut points = [Point { x: 0, y: 0}; ITEMS];
        let mut mem = super::Memory::new(&mut points);

        for _ in 0..ITEMS*2 {
            let ptr = mem.alloc_one().unwrap();
            unsafe { mem.free_one(ptr); }
        }
    }

    #[test]
    fn can_use_allocs() {
        const ITEMS: usize = 20;
        let mut points = [Point { x: 0, y: 0}; ITEMS];
        let mut mem = super::Memory::new(&mut points);
        let block = mem.alloc_one().unwrap();
        *block = Point { x: 1, y: 1};
        drop(points);
    }


    #[test]
    fn can_alloc_memory_by_ones() {
        const ITEMS: usize = 20;
        let mut points = [Point { x: 0, y: 0}; ITEMS];
        let mut mem = super::Memory::new(&mut points);

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
        let mut points = [Point { x: 0, y: 0}; ITEMS];
        let mut mem = super::Memory::new(&mut points);
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
        let mut points = [Point { x: 0, y: 0}; ITEMS];
        let mut mem = super::Memory::new(&mut points);

        let mut alloced: [Option<&mut [Point]>; BLOCKS] = [None, None, None, None, None];
        for i in 0..BLOCKS {
            alloced[i] = mem.alloc_many(SIZE);
            assert!(alloced[i].is_some());
            assert!(alloced[i].as_ref().unwrap().len() == SIZE);
            alloced[i].as_deref_mut().unwrap()[0] = Point { x: i, y: i};
            alloced[i].as_deref_mut().unwrap()[1] = Point { x: i, y: i};
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
        let mut points = [Point { x: 0, y: 0}; ITEMS];
        let mut mem = super::Memory::new(&mut points);

        let mut alloced: [Option<*mut Point>; BLOCKS] = [None, None, None, None, None];
        for i in 0..BLOCKS {
            alloced[i] = mem.alloc_many_raw(SIZE);
            assert!(alloced[i].is_some());
        }
        assert!(mem.alloc_one().is_none());

        for i in 0..BLOCKS {
            unsafe { mem.free_many(alloced[i].unwrap(), SIZE); }
        }


        for i in 0..ITEMS {
            assert!(mem.alloc_one().is_some(), "failed to alloc {i}");
        }
        assert!(mem.alloc_one().is_none());
    }
}