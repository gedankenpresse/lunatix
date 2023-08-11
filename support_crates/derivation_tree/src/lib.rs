//! Abstraction library for working with capability derivations

#![no_std]

extern crate alloc;

mod correspondence;
mod cursors;
mod node;
mod tree;

#[cfg(test)]
pub(crate) use test::assume_init_box;

#[cfg(test)]
mod test {
    extern crate std;

    use alloc::boxed::Box;
    use core::mem::MaybeUninit;

    pub unsafe fn assume_init_box<T>(value: Box<MaybeUninit<T>>) -> Box<T> {
        let raw = Box::into_raw(value);
        Box::from_raw(raw as *mut T)
    }
}
