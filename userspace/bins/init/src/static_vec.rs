use core::{
    mem::MaybeUninit,
    ptr::{addr_of, addr_of_mut},
};

pub struct StaticVec<T, const N: usize> {
    data: MaybeUninit<[T; N]>,
    length: usize,
}

#[allow(unused)]
impl<T, const N: usize> StaticVec<T, N> {
    pub fn new() -> Self {
        let this = Self {
            data: MaybeUninit::uninit(),
            length: 0,
        };
        return this;
    }

    pub const fn full(&self) -> bool {
        self.length >= N
    }

    pub const fn len(&self) -> usize {
        self.length
    }

    pub const fn as_slice(&self) -> &[T] {
        unsafe { core::slice::from_raw_parts(self.data.as_ptr().cast(), self.length) }
    }

    #[allow(unused)]
    pub fn as_slice_mut(&mut self) -> &mut [T] {
        unsafe { core::slice::from_raw_parts_mut(self.data.as_mut_ptr().cast(), self.length) }
    }

    pub unsafe fn push_unchecked(&mut self, value: T) {
        debug_assert!(!self.full(), "Maximum Capacity reached");
        unsafe {
            let field = addr_of_mut!((*self.data.as_mut_ptr())[self.length]);
            *field = value;
        }
        self.length += 1;
    }

    pub fn push(&mut self, value: T) {
        assert!(!self.full(), "Maximum Capacity reached");
        unsafe { self.push_unchecked(value) }
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.as_slice().iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.as_slice_mut().iter_mut()
    }
}

impl<T, const N: usize> core::ops::Index<usize> for StaticVec<T, N> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        assert!(index < self.length, "index out of range");
        unsafe { addr_of!((*self.data.as_ptr())[index]).as_ref().unwrap() }
    }
}

impl<T, const N: usize> core::ops::IndexMut<usize> for StaticVec<T, N> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        assert!(index < self.length, "index out of range");
        unsafe {
            addr_of_mut!((*self.data.as_mut_ptr())[index])
                .as_mut()
                .unwrap()
        }
    }
}
