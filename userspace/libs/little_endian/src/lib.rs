#![no_std]

pub trait LittleEndian {
    fn from_le(t: Self) -> Self;
    fn to_le(t: Self) -> Self;
}

macro_rules! impl_little_endian {
    ($t: ty) => {
        impl LittleEndian for $t {
            fn from_le(t: Self) -> Self {
                <$t>::from_le(t)
            }

            fn to_le(t: Self) -> Self {
                <$t>::to_le(t)
            }
        }
    };
}

impl_little_endian!(u32);
impl_little_endian!(u64);
impl_little_endian!(u16);

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct LE<T: LittleEndian>(T);

impl<T: LittleEndian + Copy + core::fmt::Debug> core::fmt::Debug for LE<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("LE").field(&T::from_le(self.0)).finish()
    }
}

impl<T: LittleEndian> LE<T> {
    pub fn new(v: T) -> Self {
        LE(T::to_le(v))
    }
}

impl<T: LittleEndian + Copy> LE<T> {
    pub fn get(&self) -> T {
        T::from_le(self.0)
    }

    pub fn set(&mut self, t: T) {
        self.0 = T::to_le(t)
    }
}

impl<T: LittleEndian + Default> Default for LE<T> {
    fn default() -> Self {
        Self::new(Default::default())
    }
}
