use core::{alloc::GlobalAlloc, cell::OnceCell};

// This is a lie, but I don't care right now
unsafe impl<A> Sync for StaticOnceCell<A> {}
unsafe impl<A> Send for StaticOnceCell<A> {}
pub struct StaticOnceCell<A> {
    cell: OnceCell<A>,
}

impl<A> StaticOnceCell<A> {
    pub const fn new() -> Self {
        Self {
            cell: OnceCell::new(),
        }
    }

    pub fn get(&self) -> Option<&A> {
        self.cell.get()
    }

    pub fn get_or_init(&self, f: impl FnOnce() -> A) -> &A {
        self.cell.get_or_init(f)
    }
}

unsafe impl<A> GlobalAlloc for StaticOnceCell<A>
where
    A: GlobalAlloc,
{
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        self.get().unwrap().alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        self.get().unwrap().dealloc(ptr, layout)
    }
}
