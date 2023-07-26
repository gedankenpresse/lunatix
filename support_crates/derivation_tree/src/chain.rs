use core::{cell::Cell, pin::Pin};

enum OwnedCell<T> {
    Owned(Option<T>),
    Ref(*const T),
    Uninit,
}

pub struct RefChain<T> {
    value: OwnedCell<T>,
    prev: Cell<*mut RefChain<T>>,
    next: Cell<*mut RefChain<T>>,
    _pin: core::marker::PhantomPinned,
}

impl<T> Drop for RefChain<T> {
    fn drop(&mut self) {
        // do we have to check drop guarantees of pin?
        let prev_ptr = self.prev.get();
        let next_ptr = self.next.get();
        if let Some(prev) = unsafe { prev_ptr.as_ref() } {
            prev.next.set(next_ptr);
        }
        if let Some(next) = unsafe { next_ptr.as_ref() } {
            next.prev.set(prev_ptr);
        }

        match &mut self.value {
            OwnedCell::Owned(t) => {
                assert!(prev_ptr.is_null());
                match unsafe { next_ptr.as_mut() } {
                    Some(next) => {
                        // we found a value to dump our value to
                        // todo: assert next.value is ref
                        next.value = OwnedCell::Owned(t.take());
                        let new_ref = match &next.value {
                            OwnedCell::Owned(Some(t)) => t as *const T,
                            _ => unreachable!(),
                        };
                        let mut others = next.next.get();
                        while let Some(other) = unsafe { others.as_mut() } {
                            // todo: asart value is ref
                            other.value = OwnedCell::Ref(new_ref);
                            others = other.next.get();
                        }
                    }
                    None => {
                        // no other nodes, drop value
                        drop(t)
                    }
                }
            }
            OwnedCell::Ref(_) => {}
            OwnedCell::Uninit => {}
        }
    }
}

impl<T> RefChain<T> {
    fn uninit() -> RefChain<T> {
        RefChain {
            value: OwnedCell::Uninit,
            prev: Cell::new(core::ptr::null_mut()),
            next: Cell::new(core::ptr::null_mut()),
            _pin: core::marker::PhantomPinned,
        }
    }

    fn new(value: T) -> RefChain<T> {
        RefChain {
            value: OwnedCell::Owned(Some(value)),
            prev: Cell::new(core::ptr::null_mut()),
            next: Cell::new(core::ptr::null_mut()),
            _pin: core::marker::PhantomPinned,
        }
    }

    fn with_val<R: 'static>(&self, f: impl FnOnce(&T) -> R) -> R {
        let val_ref = match &self.value {
            OwnedCell::Owned(Some(v)) => v,
            OwnedCell::Owned(None) => unreachable!(),
            OwnedCell::Ref(r) => unsafe { r.as_ref().unwrap() },
            OwnedCell::Uninit => panic!(),
        };
        f(val_ref)
    }

    // TODO: check if this is unsound
    // this is definetly unsound, because the value could be moved on drop, invalidating this ref
    fn try_get_val(self: &Self) -> Option<Pin<&T>> {
        match &self.value {
            // this case is may be unsound, we haven't constructed v from a pinned type
            OwnedCell::Owned(Some(v)) => Some(unsafe { Pin::new_unchecked(v) }),
            OwnedCell::Owned(None) => unreachable!(),
            OwnedCell::Ref(ptr) => Some(unsafe { Pin::new_unchecked(ptr.as_ref().unwrap()) }),
            OwnedCell::Uninit => None,
        }
    }

    fn is_uninit(self: Pin<&RefChain<T>>) -> bool {
        match self.value {
            OwnedCell::Uninit => true,
            _ => false,
        }
    }

    unsafe fn as_pointer(self: Pin<&RefChain<T>>) -> *const RefChain<T> {
        Pin::into_inner_unchecked(self) as *const RefChain<T>
    }

    fn clone_pinned<'a>(
        self: Pin<&RefChain<T>>,
        target: Pin<&'a mut RefChain<T>>,
    ) -> Pin<&'a RefChain<T>> {
        assert!(target.as_ref().is_uninit());
        assert!(target.next.get().is_null());
        assert!(target.prev.get().is_null());
        let val_ref = unsafe {
            match &Pin::into_inner_unchecked(self).value {
                OwnedCell::Owned(Some(v)) => OwnedCell::Ref(v as *const T),
                OwnedCell::Owned(None) => unreachable!(),
                OwnedCell::Ref(a) => OwnedCell::Ref(*a),
                OwnedCell::Uninit => OwnedCell::Uninit,
            }
        };

        unsafe {
            let target = Pin::into_inner_unchecked(target);
            target.next.set(self.next.get());
            target.prev.set(Self::as_pointer(self) as *mut RefChain<T>);
            match self.next.get().as_ref() {
                Some(n) => n
                    .prev
                    .set(Self::as_pointer(Pin::new_unchecked(target)) as *mut RefChain<T>),
                None => todo!(),
            }

            Pin::new_unchecked(target)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RefChain;

    #[test]
    fn can_new() {
        let chain = RefChain::new(1);
    }

    #[test]
    fn can_read() {
        let chain = RefChain::new(2);
        assert_eq!(2, chain.with_val(|v| *v));
    }

    // todo: add test for cloning
}
