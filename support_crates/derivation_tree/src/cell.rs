use core::{cell::Cell, ops::Deref, pin::Pin};

use super::refs::SharedRef;

/// This is a reference counted value without heap allocations.
/// This means that unlike core::rc::Rc, you still have to drop all references before the owner
/// Unlike normal `&` references, references to an InlineRc do not carry a lifetime.
/// This makes it suitable to use in cases where the compiler is not smart enough to figure out lifetime semantics.
///
/// As a programmer, you have to make sure that all lifetimes are correct.
/// Otherwise, the program panics if you drop an InlineRc with active references.
///
/// The corresponding reference type to an InlineRc is a InlineRef.
/// You can only create an InlineRef if you can guarrante that you won't move the InlineRc by using a Pin<&InlineRc>.
/// This restriction prevents use-after free bugs.
pub struct InlineRc<T> {
    refs: Cell<usize>,
    value: T,
    _pin: core::marker::PhantomPinned,
}

impl<T> InlineRc<T> {
    pub fn new(value: T) -> InlineRc<T> {
        InlineRc {
            refs: Cell::new(0),
            value,
            _pin: core::marker::PhantomPinned,
        }
    }

    pub fn get_ref(self: Pin<&Self>) -> InlineRef<T> {
        // increase refcount
        self.refs.set(self.refs.get() + 1);
        let shared: SharedRef<InlineRc<T>> = unsafe {
            let inner = Pin::into_inner_unchecked(self);
            SharedRef {
                value: inner as *const InlineRc<T>,
            }
        };
        InlineRef { rc: shared }
    }
}

impl<T> Drop for InlineRc<T> {
    fn drop(&mut self) {
        if self.refs.get() != 0 {
            panic!("dropped while refs present");
        }
    }
}

pub struct InlineRef<T> {
    rc: SharedRef<InlineRc<T>>,
}

impl<T> Deref for InlineRef<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &(*self.rc).value
    }
}

impl<T> AsRef<T> for InlineRef<T> {
    fn as_ref(&self) -> &T {
        &(*self.rc).value
    }
}

impl<T> Drop for InlineRef<T> {
    fn drop(&mut self) {
        let count = self.rc.refs.get();
        assert!(count > 0);
        self.rc.refs.set(count - 1);
        core::mem::forget(self.rc);
    }
}

#[cfg(test)]
mod tests {
    use core::pin::pin;
    extern crate std;

    use super::InlineRc;
    use core::cell::Cell;
    use core::convert::AsRef;
    use core::pin::Pin;
    use std::boxed::Box;

    fn rc_pinned<T>(value: T) -> Pin<Box<InlineRc<T>>> {
        Box::pin(InlineRc::new(value))
    }

    #[test]
    fn should_be_able_to_construct_inline_rc() {
        let cell = InlineRc::new(0);
    }

    #[test]
    fn should_be_able_to_get_ref() {
        let cell = rc_pinned(0);
        let reference = InlineRc::get_ref(cell.as_ref());
        drop(reference);
        drop(cell);
        let _ = Box::new(0);
    }

    #[test]
    fn can_mutate_through_ref() {
        let cell = rc_pinned(Cell::new(0));
        let reference = InlineRc::get_ref(cell.as_ref());
        let r: &_ = &*reference;
        r.set(1);
        drop(reference);
        assert_eq!(cell.value.get(), 1);
        drop(cell);
        let _ = Box::new(0);
    }

    #[should_panic]
    #[test]
    fn dropping_with_active_ref() {
        let reference = {
            let cell = InlineRc::new(0);
            let pin = pin!(cell);
            let shared_pin = pin.as_ref();
            let reference = InlineRc::get_ref(shared_pin);
            reference
        };
        drop(reference);
    }

    /*
    #[test]
    fn unsound_move() {
        let cell_a: InlineRc<_>;
        let cell_b: InlineRc<_>;

        cell_a = InlineRc::new(0);
        let pin_a_mut = pin!(cell_a);
        let pin_a = pin_a_mut.as_ref();
        let reference = InlineRc::get_ref(pin_a);
        cell_b = cell_a;
    }
    */
}
