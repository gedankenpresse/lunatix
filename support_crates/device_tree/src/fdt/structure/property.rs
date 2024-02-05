//! Handling of properties inside nodes

use crate::fdt::structure::buf_tools::ByteSliceWithTokens;
use crate::fdt::structure::FDT_PROP;
use crate::fdt::{Strings, StringsError};
use core::ffi::CStr;
use thiserror_no_std::Error;

#[derive(Debug, Error, Eq, PartialEq)]
pub enum PropertyParseError {
    #[error("The property referenced a string for its name that could not be fetched")]
    NameError(#[from] StringsError),
    #[error("The given buffer of size {0} is not large enough to contain a property and its value (12 bytes header + {1:?} bytes value according to the header)")]
    BufferTooSmall(usize, Option<usize>),
    #[error("The given buffer does not start with an FDT_PROP token")]
    NotAProp,
}

/// A single property inside a node
#[derive(Debug, Eq, PartialEq)]
pub struct NodeProperty<'buf> {
    /// Offset to the name of the property into the *strings* block of the FDT
    pub name: &'buf CStr,
    /// Value of the property
    pub value: &'buf [u8],
}

impl<'buf> NodeProperty<'buf> {
    /// Parse a property from the given buffer and returns that property in addition to how many bytes of the buffer
    /// are used by it.
    ///
    /// The buffer must start immediately with the `FDT_PROP` token but may be larger than the one property.
    pub(super) fn from_buffer(
        buf: &'buf [u8],
        strings: &Strings<'buf>,
    ) -> Result<(usize, Self), PropertyParseError> {
        // check preconditions
        if !matches!(buf.next_token(true), Some((0, FDT_PROP))) {
            return Err(PropertyParseError::NotAProp);
        }
        if buf.len() < 12 {
            return Err(PropertyParseError::BufferTooSmall(buf.len(), None));
        }

        // parse header and resolve name
        let value_len = u32::from_be_bytes([buf[4], buf[5], buf[6], buf[7]]);
        let name_offset = u32::from_be_bytes([buf[8], buf[9], buf[10], buf[11]]);
        let name = strings.get_string(name_offset as usize)?;

        // fetch property value from buffer and return result
        match buf.get(12..12 + value_len as usize) {
            None => Err(PropertyParseError::BufferTooSmall(
                buf.len(),
                Some(value_len as usize),
            )),
            Some(value) => Ok((12 + value_len as usize, Self { name, value })),
        }
    }
}

/// An iterator over node properties that are encoded in a buffer
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct PropertyIter<'buf> {
    pub(super) buf: Option<&'buf [u8]>,
    pub(super) strings: Strings<'buf>,
}

impl<'buf> PropertyIter<'buf> {
    pub(super) fn new(buf: &'buf [u8], strings: Strings<'buf>) -> Self {
        Self {
            strings,
            buf: if buf.len() == 0 { None } else { Some(buf) },
        }
    }
}

impl<'buf> Iterator for PropertyIter<'buf> {
    type Item = NodeProperty<'buf>;

    fn next(&mut self) -> Option<Self::Item> {
        let buf = self.buf?;
        let (prop_len, prop) = NodeProperty::from_buffer(buf, &self.strings).ok()?;
        self.buf = buf.get(prop_len..);
        Some(prop)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn from_buffer_works_with_zero_size_value() {
        let strings = Strings::from_buffer(b"/test\0");
        let mut buf = [0u8; 16];
        buf[0..4].copy_from_slice(&FDT_PROP.to_be_bytes());
        buf[4..8].copy_from_slice(&0u32.to_be_bytes()); // len
        buf[8..12].copy_from_slice(&0u32.to_be_bytes()); // name offset

        let (prop_size, prop) = NodeProperty::from_buffer(&buf, &strings).unwrap();
        assert_eq!(prop_size, 12);
        assert_eq!(prop.name.to_str().unwrap(), "/test");
        assert!(prop.value.is_empty());
    }

    #[test]
    fn from_buffer_works_with_u64_value() {
        let strings = Strings::from_buffer(b"/test\0");
        let mut buf = [0u8; 32];
        buf[0..4].copy_from_slice(&FDT_PROP.to_be_bytes());
        buf[4..8].copy_from_slice(&8u32.to_be_bytes()); // len
        buf[8..12].copy_from_slice(&0u32.to_be_bytes()); // name offset
        buf[12..20].copy_from_slice(&(!0u64).to_be_bytes()); // value

        let (prop_size, prop) = NodeProperty::from_buffer(&buf, &strings).unwrap();
        assert_eq!(prop_size, 20);
        assert_eq!(prop.name.to_str().unwrap(), "/test");
        assert_eq!(prop.value, &(!0u64).to_be_bytes());
    }

    #[test]
    fn iterator_works_with_one_prop() {
        let strings = Strings::from_buffer(b"/test\0");
        let mut buf = [0u8; 32];
        buf[0..4].copy_from_slice(&FDT_PROP.to_be_bytes());
        buf[4..8].copy_from_slice(&8u32.to_be_bytes()); // len
        buf[8..12].copy_from_slice(&0u32.to_be_bytes()); // name offset
        buf[12..20].copy_from_slice(&(!0u64).to_be_bytes()); // value

        let iter = PropertyIter {
            strings,
            buf: Some(&buf),
        };
        assert_eq!(iter.clone().count(), 1);
        assert_eq!(iter.clone().nth(0).unwrap().name.to_str().unwrap(), "/test");
        assert_eq!(iter.clone().nth(0).unwrap().value, &(!0u64).to_be_bytes());
    }

    #[test]
    fn iterator_works_with_two_props() {
        let strings = Strings::from_buffer(b"/test\0");
        let mut buf = [0u8; 64];
        buf[0..4].copy_from_slice(&FDT_PROP.to_be_bytes());
        buf[4..8].copy_from_slice(&8u32.to_be_bytes()); // len
        buf[8..12].copy_from_slice(&0u32.to_be_bytes()); // name offset
        buf[12..20].copy_from_slice(&(!0u64).to_be_bytes()); // value

        buf[20..24].copy_from_slice(&FDT_PROP.to_be_bytes());
        buf[24..28].copy_from_slice(&8u32.to_be_bytes()); // len
        buf[28..32].copy_from_slice(&0u32.to_be_bytes()); // name offset
        buf[32..40].copy_from_slice(&0xABABABu64.to_be_bytes()); // value

        let iter = PropertyIter {
            strings,
            buf: Some(&buf),
        };
        assert_eq!(iter.clone().count(), 2);
        assert_eq!(iter.clone().nth(0).unwrap().name.to_str().unwrap(), "/test");
        assert_eq!(iter.clone().nth(0).unwrap().value, &(!0u64).to_be_bytes());
        assert_eq!(iter.clone().nth(1).unwrap().name.to_str().unwrap(), "/test");
        assert_eq!(
            iter.clone().nth(1).unwrap().value,
            &0xABABABu64.to_be_bytes()
        );
    }
}
