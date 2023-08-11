use core::cell::Cell;
use core::marker::PhantomPinned;
use core::pin::Pin;

pub struct Link<T> {
    elem: T,
    prev: Cell<*mut Link<T>>,
    next: Cell<*mut Link<T>>,
    collection: *const CursorSet<T>,
    _pin: PhantomPinned,
}

pub struct CursorSet<T> {
    init: Cell<*const Cursor<T>>,
    _pin: PhantomPinned,
}

impl<T> Drop for CursorSet<T> {
    fn drop(&mut self) {
        if !self.init.get().is_null() {
            panic!("dropped cursor set with active cursors");
        }
    }
}

impl<T> CursorSet<T> {
    fn invalidate(&self, this: *const Link<T>) {
        let mut cur = self.init.get();
        while let Some(next) = unsafe { cur.as_ref() } {
            next.invalidate(this);
            cur = next.next.get();
        }
        cur = self.init.get();
        while let Some(prev) = unsafe { cur.as_ref() } {
            prev.invalidate(this);
            cur = prev.prev.get();
        }
    }

    const fn empty() -> Self {
        CursorSet {
            init: Cell::new(core::ptr::null()),
            _pin: PhantomPinned,
        }
    }
}

pub struct Cursor<T> {
    elem: Cell<*mut Link<T>>,
    prev: Cell<*const Cursor<T>>,
    next: Cell<*const Cursor<T>>,
    set: Cell<*const CursorSet<T>>,
    _pin: PhantomPinned,
}

impl<T> Drop for Cursor<T> {
    fn drop(&mut self) {
        inner_drop(unsafe { Pin::new_unchecked(self) });
        fn inner_drop<T>(this: Pin<&mut Cursor<T>>) {
            this.as_ref().unlink();
        }
    }
}

impl<T> Cursor<T> {
    fn invalidate(&self, this: *const Link<T>) {
        if self.elem.get() as *const _ == this {
            self.elem.set(core::ptr::null_mut());
            panic!("deleted node with active cursor");
        }
    }

    /// get another sibling cursor, if it exists
    fn get_other(self: Pin<&Self>) -> *const Cursor<T> {
        let prev = self.prev.get();
        if !prev.is_null() {
            return prev;
        }

        let next = self.next.get();
        if !next.is_null() {
            return next;
        }
        return core::ptr::null();
    }

    fn unlink(self: Pin<&Self>) {
        let prev = self.prev.get();
        let next = self.next.get();
        unsafe {
            if let Some(prev) = prev.as_ref() {
                prev.next.set(next);
            }
            if let Some(next) = next.as_ref() {
                next.prev.set(prev);
            }
        }

        if let Some(set) = unsafe { self.set.get().as_ref() } {
            set.init.set(self.get_other());
        }

        self.set.set(core::ptr::null());
        self.prev.set(core::ptr::null_mut());
        self.next.set(core::ptr::null_mut());
    }
}

impl<T> Drop for Link<T> {
    fn drop(&mut self) {
        #[cfg(test)]
        {
            extern crate std;
            std::println!("dropping: {:p}", self);
        }
        inner_drop(unsafe { Pin::new_unchecked(self) });
        fn inner_drop<T>(this: Pin<&mut Link<T>>) {
            this.as_ref().unlink();
        }
    }
}

impl<T> Link<T> {
    fn new(value: T) -> Self {
        Link {
            elem: value,
            prev: Cell::new(core::ptr::null_mut()),
            next: Cell::new(core::ptr::null_mut()),
            collection: core::ptr::null_mut(),
            _pin: PhantomPinned,
        }
    }

    unsafe fn as_pointer(self: Pin<&Self>) -> *const Self {
        Pin::into_inner_unchecked(self) as *const Self
    }

    unsafe fn as_mut_pointer(self: Pin<&mut Self>) -> *mut Self {
        Pin::into_inner_unchecked(self) as *mut Self
    }

    unsafe fn as_pin<'a>(this: *const Self) -> Pin<&'a Self> {
        Pin::new_unchecked(&*this)
    }

    unsafe fn as_pin_mut<'a>(this: *mut Self) -> Pin<&'a mut Self> {
        Pin::new_unchecked(&mut *this)
    }

    fn unlink(self: Pin<&Self>) {
        unsafe {
            let this = self.as_pointer();
            let prev = (*this).prev.get();
            let next = (*this).next.get();
            if !prev.is_null() {
                #[cfg(test)]
                {
                    extern crate std;
                    std::println!("unlinking from: {:p}", prev);
                }
                (*prev).next.set(next);
            }
            if !next.is_null() {
                #[cfg(test)]
                {
                    extern crate std;
                    std::println!("unlinking from: {:p}", next);
                }
                (*next).prev.set(prev);
            }
            (*this).prev.set(core::ptr::null_mut());
            (*this).next.set(core::ptr::null_mut());
            let collection = (*this).collection;
            if !collection.is_null() {
                (*collection).invalidate(this as *mut _);
            }
        }
    }

    fn set_collection(self: Pin<&mut Link<T>>, collection: *const CursorSet<T>) {
        unsafe {
            Pin::into_inner_unchecked(self).collection = collection;
        }
    }

    /// Insert `link` after `self`
    fn link_after(self: Pin<&Self>, mut link: Pin<&mut Self>) {
        assert!(link.collection.is_null());
        link.as_mut().set_collection(self.collection);
        let link_ptr = unsafe { link.as_mut_pointer() };
        if let Some(next) = unsafe { self.next.get().as_ref() } {
            next.prev.set(link_ptr);
        }
        self.next.set(link_ptr);
    }

    /// Insert `link` before `self`
    fn link_before(self: Pin<&Self>, mut link: Pin<&mut Self>) {
        assert!(link.collection.is_null());
        link.as_mut().set_collection(self.collection);
        let link_ptr = unsafe { link.as_mut_pointer() };
        if let Some(prev) = unsafe { self.prev.get().as_ref() } {
            prev.next.set(link_ptr);
        }
        self.prev.set(link_ptr);
    }
}

#[macro_export]
macro_rules! link {
    ($v:expr) => {{
        let mut v = $crate::intrusive::linked_list::Link::new($v);
        core::pin::pin!(v)
    }};
}

#[cfg(test)]
mod tests {
    use core::pin::Pin;

    extern crate std;
    use super::CursorSet;

    #[test]
    fn can_create_set() {
        let set: CursorSet<usize> = CursorSet::empty();
    }

    #[test]
    fn can_create_link() {
        let link = link!(1);
        drop(link);
    }

    #[test]
    fn can_create_links() {
        let mut linkb = link!(1);
        let mut linka = link!(2);
        linka.as_ref().link_after(linkb.as_mut());
        std::println!("drop a");
        drop(linka);
        assert!(linkb.prev.get().is_null());
        assert!(linkb.next.get().is_null());
        std::println!("drop b");
        drop(linkb);
    }
}
