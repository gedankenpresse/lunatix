//! Handling of properties inside nodes

use crate::fdt::structure::buf_tools::{align_to_token, ByteSliceWithTokens};
use crate::fdt::structure::property_value_encoding::{
    InvalidValueLength, StringError, StringListIterator,
};
use crate::fdt::structure::FDT_PROP;
use crate::fdt::{Strings, StringsError};
use core::mem;
use thiserror_no_std::Error;

/// The error which can occur when parsing a property
#[derive(Debug, Error, Eq, PartialEq)]
pub enum PropertyParseError {
    /// The property referenced a string for its name that could not be fetched
    #[error("The property referenced a string for its name that could not be fetched")]
    NameError(#[from] StringsError),
    /// The given underlying buffer is not large enough to contain the property value according to the property header
    #[error("The given buffer of size {0} is not large enough to contain a property and its value (12 bytes header + {1:?} bytes value according to the header)")]
    BufferTooSmall(usize, Option<usize>),
    /// The given buffer does not start with an FDT_PROP token and is therefore not a property
    #[error("The given buffer does not start with an FDT_PROP token")]
    NotAProp,
}

/// A single property inside a node
#[derive(Debug, Eq, PartialEq)]
pub struct NodeProperty<'buf> {
    /// Offset to the name of the property into the *strings* block of the FDT
    pub name: &'buf str,
    /// Value of the property
    pub value: &'buf [u8],
}

impl<'buf> NodeProperty<'buf> {
    /// Parse a property from the given buffer and returns that property in addition to how many bytes of the buffer
    /// are used by it.
    ///
    /// The buffer must start immediately with the `FDT_PROP` token but may be larger than the one property.
    pub fn from_buffer(
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

    /// Interpret the value of this property as a single u32
    ///
    /// # Example
    ///
    /// ```rust
    /// # use device_tree::fdt::{NodeProperty, Strings};
    /// # let strings = Strings::from_buffer(b"\0");
    /// # let mut buf = [0u8; 128];
    /// # buf[0..4].copy_from_slice(&0x00000003u32.to_be_bytes());
    /// # buf[4..8].copy_from_slice(&4u32.to_be_bytes()); // len
    /// # buf[8..12].copy_from_slice(&0u32.to_be_bytes()); // name offset
    /// # buf[12..16].copy_from_slice(&42u32.to_be_bytes()); // value
    /// #
    /// # let (_, prop) = NodeProperty::from_buffer(&buf, &strings).unwrap();
    /// let value = prop.as_u32();
    /// assert_eq!(value, Ok(42u32));
    /// ```
    pub fn as_u32(&self) -> Result<u32, InvalidValueLength> {
        self.try_into()
    }

    /// Assuming the property encodes a list of u32 values, read the nth one
    pub fn nth_u32(&self, n: usize) -> Result<u32, InvalidValueLength> {
        let buf = self
            .value
            .get(n * mem::size_of::<u32>()..(n + 1) * mem::size_of::<u32>())
            .ok_or(InvalidValueLength)?
            .try_into()
            .unwrap();
        Ok(u32::from_be_bytes(buf))
    }

    /// Interpret the value of this property as a single u32
    ///
    /// # Example
    ///
    /// ```rust
    /// # use device_tree::fdt::{NodeProperty, Strings};
    /// # let strings = Strings::from_buffer(b"\0");
    /// # let mut buf = [0u8; 128];
    /// # buf[0..4].copy_from_slice(&0x00000003u32.to_be_bytes());
    /// # buf[4..8].copy_from_slice(&4u32.to_be_bytes()); // len
    /// # buf[8..12].copy_from_slice(&0u32.to_be_bytes()); // name offset
    /// # buf[12..16].copy_from_slice(&42u32.to_be_bytes()); // value
    /// #
    /// # let (_, prop) = NodeProperty::from_buffer(&buf, &strings).unwrap();
    /// let value = prop.as_phandle();
    /// assert_eq!(value, Ok(42));
    /// ```
    pub fn as_phandle(&self) -> Result<u32, InvalidValueLength> {
        self.try_into()
    }

    /// Interpret the value of this property as a single u64
    ///
    /// # Example
    ///
    /// ```rust
    /// # use device_tree::fdt::{NodeProperty, Strings};
    /// # let strings = Strings::from_buffer(b"\0");
    /// # let mut buf = [0u8; 128];
    /// # buf[0..4].copy_from_slice(&0x00000003u32.to_be_bytes());
    /// # buf[4..8].copy_from_slice(&8u32.to_be_bytes()); // len
    /// # buf[8..12].copy_from_slice(&0u32.to_be_bytes()); // name offset
    /// # buf[12..20].copy_from_slice(&42u64.to_be_bytes()); // value
    /// #
    /// # let (_, prop) = NodeProperty::from_buffer(&buf, &strings).unwrap();
    /// let value = prop.as_u64();
    /// assert_eq!(value, Ok(42u64));
    /// ```
    pub fn as_u64(&self) -> Result<u64, InvalidValueLength> {
        self.try_into()
    }

    /// Assuming the property encodes a list of u64 values, read the nth one
    pub fn nth_u64(&self, n: usize) -> Result<u64, InvalidValueLength> {
        let buf = self
            .value
            .get(n * mem::size_of::<u64>()..(n + 1) * mem::size_of::<u64>())
            .ok_or(InvalidValueLength)?
            .try_into()
            .unwrap();
        Ok(u64::from_be_bytes(buf))
    }

    /// Interpret the value of this property as a single null-terminated string
    ///
    /// # Example
    ///
    /// ```rust
    /// # use device_tree::fdt::{NodeProperty, Strings};
    /// # let strings = Strings::from_buffer(b"\0");
    /// # let mut buf = [0u8; 128];
    /// # buf[0..4].copy_from_slice(&0x00000003u32.to_be_bytes());
    /// # buf[4..8].copy_from_slice(&6u32.to_be_bytes()); // len
    /// # buf[8..12].copy_from_slice(&0u32.to_be_bytes()); // name offset
    /// # buf[12..18].copy_from_slice(b"hello\0"); // value
    /// #
    /// # let (_, prop) = NodeProperty::from_buffer(&buf, &strings).unwrap();
    /// let value = prop.as_string();
    /// assert_eq!(value, Ok("hello"));
    /// ```
    pub fn as_string(&self) -> Result<&'buf str, StringError> {
        self.try_into()
    }

    /// Interpret the value of this property as a list of null-terminated strings
    ///
    /// # Example
    ///
    /// ```rust
    /// # use device_tree::fdt::{NodeProperty, Strings};
    /// # let strings = Strings::from_buffer(b"\0");
    /// # let mut buf = [0u8; 128];
    /// # buf[0..4].copy_from_slice(&0x00000003u32.to_be_bytes());
    /// # buf[4..8].copy_from_slice(&12u32.to_be_bytes()); // len
    /// # buf[8..12].copy_from_slice(&0u32.to_be_bytes()); // name offset
    /// # buf[12..24].copy_from_slice(b"hello\0world\0"); // value
    /// #
    /// # let (_, prop) = NodeProperty::from_buffer(&buf, &strings).unwrap();
    /// let mut value = prop.as_string_list();
    /// assert_eq!(value.next_str(), Ok("hello"));
    /// assert_eq!(value.next_str(), Ok("world"));
    /// ```
    pub fn as_string_list(&self) -> StringListIterator<'buf> {
        StringListIterator { buf: self.value }
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
        self.buf = buf.get(align_to_token(prop_len)..);
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
        assert_eq!(prop.name, "/test");
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
        assert_eq!(prop.name, "/test");
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
        assert_eq!(iter.clone().nth(0).unwrap().name, "/test");
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
        assert_eq!(iter.clone().nth(0).unwrap().name, "/test");
        assert_eq!(iter.clone().nth(0).unwrap().value, &(!0u64).to_be_bytes());
        assert_eq!(iter.clone().nth(1).unwrap().name, "/test");
        assert_eq!(
            iter.clone().nth(1).unwrap().value,
            &0xABABABu64.to_be_bytes()
        );
    }
}
