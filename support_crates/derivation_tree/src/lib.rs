#![no_std]


#[cfg(feature = "std")]
extern crate std;

use core::cell;
use cell::UnsafeCell;
use cell::RefCell;
use cell::Cell;

pub enum HereThere<T> {
    Here(RefCell<T>),
    There(*const Slot<T>),
    Uninit
}

// there should be some manually drop here somewhere
pub struct Slot<T> {
    inner: UnsafeCell<HereThere<T>>,
    prev: Cell<*mut Slot<T>>,
    next: Cell<*mut Slot<T>>,
    depth: Cell<usize>,
}



impl<T> Slot<T> {
    pub const fn uninit() -> Self {
        Self {
            inner: UnsafeCell::new(HereThere::Uninit),
            prev: Cell::new(core::ptr::null_mut()),
            next: Cell::new(core::ptr::null_mut()),
            depth: Cell::new(0),
        }
    }

    pub fn new(val: T) -> Self {
        Self {
            inner: UnsafeCell::new(HereThere::Here(RefCell::new(val))),
            prev: Cell::new(core::ptr::null_mut()),
            next: Cell::new(core::ptr::null_mut()),
            depth: Cell::new(0),
        }
    }

    pub fn is_uninit(&self) -> bool {
        match unsafe { &*self.inner.get() } {
            HereThere::Uninit => true,
            _ => false,
        }
    }

    fn get_here(&self) -> Option<&RefCell<T>> {
        match unsafe { &(*self.inner.get()) } {
            HereThere::Here(here) => Some(here),
            _=> None,
        }
    }

    pub fn get(&self) -> &RefCell<T> {
        match unsafe { & *self.inner.get() } {
            HereThere::Here(here) => here,
            HereThere::There(ptr) => {
                let there = unsafe { ptr.as_ref().unwrap() };
                there.get_here().unwrap()
            },
            HereThere::Uninit => panic!("can't get uninit"),
        }
    }

    pub fn set(&self, val: T) {
        let inner = unsafe { &mut *self.inner.get() };
        match inner {
            HereThere::Here(_) => panic!("set on present value"),
            HereThere::There(_) => panic!("set on present value"),
            HereThere::Uninit => { *inner = HereThere::Here(RefCell::new(val)) },
        }
    }

    pub fn copy_link<'a>(&'a self, target: &'a Slot<T>) {
        assert!(!self.is_uninit());
        assert!(target.is_uninit());

        unsafe {
            if !self.next.get().is_null() {
                (*self.next.get()).prev.set(target as  *const Slot<T> as *mut Slot<T>); 
            }

            target.next.set(self.next.get());
            target.depth.set(self.depth.get());
            self.next.set(target as *const Slot<T> as *mut Slot<T>);
            target.prev.set(self as *const Slot<T> as *mut Slot<T>);
        }
    }

    pub fn copy_value(&self, target: &Slot<T>) {
        let val: *const Slot<T> = match unsafe{ &*self.inner.get() } {
            HereThere::Here(_) => (self as *const Slot<T> as *mut Slot<T>).cast(),
            HereThere::There(ptr) => (*ptr).cast(),
            HereThere::Uninit => unreachable!(),
        };
        unsafe { *target.inner.get() = HereThere::There(val) };
    }

    pub fn derive_link<'a>(&'a self, child: &'a Slot<T>) {
        unsafe { 
            let parent = self.get_last_copy();
            parent.as_mut().unwrap().copy_link(child);
            child.depth.set(child.depth.get() + 1);
        }
    }

    pub fn derive_with<'a>(&'a mut self, child: &'a mut Slot<T>, f: impl FnOnce(& RefCell<T>) -> T) {
        unsafe { 
            let parent = self.get_last_copy();
            parent.as_mut().unwrap().copy_link(child);
            child.depth.set(child.depth.get() + 1);
        }
        *child.inner.get_mut() = HereThere::Here(RefCell::new(f(self.get())))
    }

    fn get_last_copy(& self) -> *mut Slot<T> {
        unsafe {
            let mut cur = self as *const Slot<T> as *mut Slot<T>; 
            loop {
                let next = (*cur).next.get();
                if next.is_null() {
                    break;
                }

                // if next  points to a slot which isn't a copy of self, break
                match *(*next).inner.get() {
                    HereThere::Here(_) => break,
                    HereThere::There(ptr) => if ptr != self as *const Slot<T> { break; },
                    HereThere::Uninit => break,
                }

                cur = next;
            }
            return cur;
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    extern crate std;
    use std::boxed::Box;

    struct Cons<T> {
        car: Slot<T>,
        cdr: Slot<T>,
    }

    impl<T> Cons<T> {
        fn empty() -> Self {
            Self {
                car: Slot::uninit(),
                cdr: Slot::uninit(),
            }
        }
    }

    struct Val(Option<Box<Cons<Val>>>);

    #[test]
    fn it_can_create_uninit() {
        let a: Slot<()> = Slot::uninit();
        assert!(a.is_uninit());
    }

    #[test]
    fn it_can_create_new() {
        let a = Slot::new(1);
    }


    #[test]
    fn it_can_get_new() {
        let a = Slot::new(1);
        assert_eq!(*a.get().borrow(), 1);
    }

    #[test]
    fn it_can_copy() {
        let mut a = Slot::new(1);
        let mut b =  Slot::uninit();
        a.copy(&mut b);
        assert_eq!(*b.get().borrow(), 1);
    }

    #[test]
    fn it_can_derive() {
        let mut a = Slot::new(1);
        let mut b =  Slot::uninit();
        a.derive_with(&mut b, |v| *v.borrow() + 1);
        assert_eq!(*b.get().borrow(), 2);
    }

    #[test]
    fn it_can_map_self() {
        let mut slot: Slot<Val> = Slot::new(Val(Some(Box::new(Cons::empty()))));
        slot.copy(&slot.get().borrow().0.as_ref().unwrap().car);
    }

    #[test]
    fn it_can_map_self_and_drop() {
        let slot: Slot<Val> = Slot::new(Val(Some(Box::new(Cons::empty()))));
        let slotb = Slot::uninit();
        slot.copy(&slotb);
        slot.copy(&slot.get().borrow().0.as_ref().unwrap().car);
        drop(slot);
        let val = slotb.get().borrow();
    }
}
