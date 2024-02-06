use crate::fdt::NodeProperty;
use core::ffi::CStr;
use thiserror_no_std::Error;

#[derive(Debug, Error, Eq, PartialEq)]
#[error("The raw property value had an invalid length")]
pub struct InvalidValueLength;

#[derive(Debug, Error, Eq, PartialEq)]
pub enum StringError {
    #[error("The raw property value is not null-terminated")]
    NoNullTerminator,
    #[error("The raw property is not valid UTF-8")]
    Utf8Error,
}

impl<'buf> TryFrom<&NodeProperty<'buf>> for u32 {
    type Error = InvalidValueLength;

    fn try_from(value: &NodeProperty<'buf>) -> Result<Self, Self::Error> {
        let bytes = value.value.try_into().map_err(|_| InvalidValueLength)?;
        Ok(u32::from_be_bytes(bytes))
    }
}

impl<'buf> TryFrom<&NodeProperty<'buf>> for u64 {
    type Error = InvalidValueLength;

    fn try_from(value: &NodeProperty<'buf>) -> Result<Self, Self::Error> {
        let bytes = value.value.try_into().map_err(|_| InvalidValueLength)?;
        Ok(u64::from_be_bytes(bytes))
    }
}

impl<'buf> TryFrom<&NodeProperty<'buf>> for &'buf str {
    type Error = StringError;

    fn try_from(value: &NodeProperty<'buf>) -> Result<Self, Self::Error> {
        let cstr =
            CStr::from_bytes_with_nul(value.value).map_err(|_| StringError::NoNullTerminator)?;
        let str = cstr.to_str().map_err(|_| StringError::Utf8Error)?;
        Ok(str)
    }
}

/// An iterator over a property value that is `<stringlist>` encoded.
///
/// While `Iterator<Item = &str>` is implemented for this, parsing errors are not surfaced due to limitations
/// of the iterator api.
/// If reading the error is desired use [`next_str()`](StringListIterator::next_str) instead.
pub struct StringListIterator<'buf> {
    pub(crate) buf: &'buf [u8],
}

impl<'buf> StringListIterator<'buf> {
    /// Try to read the next string from the underlying buffer
    ///
    /// If reading fails, return a descriptive error instead.
    /// Note that the internal buffer is still advanced if possible even if an error is returned so that strings located later in the list can still be read.
    pub fn next_str(&mut self) -> Result<&'buf str, StringError> {
        let cstr =
            CStr::from_bytes_until_nul(self.buf).map_err(|_| StringError::NoNullTerminator)?;
        self.buf = &self.buf[cstr.to_bytes_with_nul().len()..];
        let str = cstr.to_str().map_err(|_| StringError::Utf8Error)?;
        Ok(str)
    }
}

impl<'buf> Iterator for StringListIterator<'buf> {
    type Item = &'buf str;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_str().ok()
    }
}
