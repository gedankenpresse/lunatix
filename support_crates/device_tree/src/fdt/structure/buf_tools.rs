use crate::fdt::structure::{FDT_BEGIN_NODE, FDT_END, FDT_END_NODE, FDT_NOP, FDT_PROP};
use core::mem;

/// A trait for working with `&[u8]` slices that contain FDT tokens
pub trait ByteSliceWithTokens {
    /// Return the next valid token from the buffer along with its position in the buffer as `(pos, token)` tuple
    fn next_token(&self, skip_nops: bool) -> Option<(usize, u32)>;

    /// Find the next instance of `token` in the slice and return the buffer index at which it starts
    fn find_token(&self, token: u32) -> Option<usize>;
}

impl<'a> ByteSliceWithTokens for &'a [u8] {
    fn next_token(&self, skip_nops: bool) -> Option<(usize, u32)> {
        for i in (0..self.len()).step_by(mem::size_of::<u32>()) {
            let i_token = u32::from_be_bytes([
                *self.get(i)?,
                *self.get(i + 1)?,
                *self.get(i + 2)?,
                *self.get(i + 3)?,
            ]);
            if [FDT_BEGIN_NODE, FDT_END_NODE, FDT_PROP, FDT_END].contains(&i_token) {
                return Some((i, i_token));
            }
            if !skip_nops && i_token == FDT_NOP {
                return Some((i, i_token));
            }
        }

        None
    }

    fn find_token(&self, token: u32) -> Option<usize> {
        let mut skip = 0;
        while let Some((i, next_token)) = (&self[skip..]).next_token(false) {
            if next_token == token {
                return Some(skip + i);
            } else {
                skip += i + 4;
            }
        }

        None
    }
}

/// Align a number (typically a buffer index) so that it can be used to access aligned FDT tokens
#[inline]
pub(crate) const fn align_to_token(n: usize) -> usize {
    const ALIGNMENT: usize = mem::align_of::<u32>();
    (n + ALIGNMENT - 1) & !(ALIGNMENT - 1)
}
