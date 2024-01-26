//! Handling of the *strings* block
//!
//! This is implemented according to [Spec Section 5.5](https://devicetree-specification.readthedocs.io/en/v0.3/flattened-format.html#strings-block).

use core::ffi::CStr;
use thiserror_no_std::Error;

#[derive(Debug, Eq, PartialEq)]
pub struct Strings<'buf> {
    buf: &'buf [u8],
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum StringsError {
    #[error("No string could be found at offset {0} in buffer of size {1}")]
    OutOfBounds(usize, usize),
    #[error("There was data at offset {0} but it was not zero-terminated")]
    Unterminated(usize),
}

impl<'buf> Strings<'buf> {
    pub fn from_buffer(buf: &'buf [u8]) -> Self {
        Self { buf }
    }

    pub fn get_string(&self, offset: usize) -> Result<&'buf CStr, StringsError> {
        match self.buf.get(offset..) {
            None => Err(StringsError::OutOfBounds(offset, self.buf.len())),
            Some(res) => match CStr::from_bytes_until_nul(res) {
                Err(_) => Err(StringsError::Unterminated(offset)),
                Ok(s) => Ok(s),
            },
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use alloc::ffi::CString;

    #[test]
    fn get_string_works() {
        let s_original = CString::new("hello world").unwrap();
        let strings = Strings::from_buffer(s_original.as_bytes_with_nul());
        let s2 = strings.get_string(0).unwrap();
        assert_eq!(s_original.as_c_str(), s2);
        assert_eq!("hello world", s2.to_str().unwrap())
    }
}
