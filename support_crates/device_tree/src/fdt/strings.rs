//! Handling of the *strings* block
//!
//! This is implemented according to [Spec Section 5.5](https://devicetree-specification.readthedocs.io/en/v0.3/flattened-format.html#strings-block).

use core::ffi::CStr;
use thiserror_no_std::Error;

/// A collection of various strings that are used throughout the fdt
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct Strings<'buf> {
    buf: &'buf [u8],
}

/// The error which can occur when looking up strings
#[derive(Debug, Error, Eq, PartialEq)]
pub enum StringsError {
    /// A string could not be looked up because the lookup index was out of bounds
    #[error("No string could be found at offset {0} in buffer of size {1}")]
    OutOfBounds(usize, usize),
    /// The data located at a given offset had no zero-terminator which is required for strings
    #[error("There was data at offset {0} but it was not zero-terminated")]
    Unterminated(usize),
    /// The string data did not contain valid UTF-8
    #[error("The string data did not contain valid UTF-8")]
    InvalidUtf8,
}

impl<'buf> Strings<'buf> {
    /// Use a buffer as strings block
    pub fn from_buffer(buf: &'buf [u8]) -> Self {
        Self { buf }
    }

    /// Try to lookup a string from the strings block
    pub fn get_string(&self, offset: usize) -> Result<&'buf str, StringsError> {
        let buf = self
            .buf
            .get(offset..)
            .ok_or(StringsError::OutOfBounds(offset, self.buf.len()))?;
        let cstr =
            CStr::from_bytes_until_nul(buf).map_err(|_| StringsError::Unterminated(offset))?;
        let str = cstr.to_str().map_err(|_| StringsError::InvalidUtf8)?;
        Ok(str)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    extern crate alloc;

    #[test]
    fn get_string_works() {
        let strings = Strings::from_buffer(b"hello world\0");
        let s2 = strings.get_string(0).unwrap();
        assert_eq!("hello world", s2);
    }
}
