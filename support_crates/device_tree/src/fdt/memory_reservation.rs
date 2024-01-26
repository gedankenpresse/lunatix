use core::mem;
use thiserror_no_std::Error;

/// A single entry of the memory reservation block.
///
/// Each entry gives the physical address and size in bytes of a reserved memory region.
/// These given regions are required to not overlap each other.
/// The list of reserved blocks shall be terminated with an entry where both address and size are equal to 0.
#[derive(Debug, Eq, PartialEq)]
pub struct MemoryReservationEntry {
    address: u64,
    size: u64,
}

/// The error which indicates that a block of memory has an invalid format to be a valid memory allocation block
#[derive(Debug, Error, Eq, PartialEq)]
pub enum FormatError {
    #[error("The memory reservation block is not aligned to an 8-byte boundary")]
    InvalidAlignment,
    #[error("The memory reservation blocks length is not a multiple of 16")]
    InvalidLength,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MemoryReservationBlock<'buf> {
    buf: &'buf [u8],
}

impl<'buf> MemoryReservationBlock<'buf> {
    pub fn from_buffer(buf: &'buf [u8]) -> Result<Self, FormatError> {
        if buf.as_ptr() as usize % 8 != 0 {
            return Err(FormatError::InvalidAlignment);
        }
        if buf.len() % (mem::size_of::<u64>() * 2) != 0 {
            return Err(FormatError::InvalidLength);
        }

        Ok(Self { buf })
    }

    fn read_u64(&mut self) -> Option<u64> {
        if self.buf.len() < mem::size_of::<u64>() {
            return None;
        }

        let (head, tail) = self.buf.split_at(mem::size_of::<u64>());
        let value = u64::from_be_bytes(head.try_into().unwrap());
        self.buf = tail;
        Some(value)
    }
}

impl<'buf> Iterator for MemoryReservationBlock<'buf> {
    type Item = MemoryReservationEntry;

    fn next(&mut self) -> Option<Self::Item> {
        Some(MemoryReservationEntry {
            address: self.read_u64()?,
            size: self.read_u64()?,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use alloc::vec;
    use alloc::vec::Vec;

    #[repr(C, align(8))]
    pub struct AlignedBuffer<const LENGTH: usize>(pub [u8; LENGTH]);

    #[test]
    fn memory_reservation_iteration_works_if_valid() {
        let mut buf = AlignedBuffer([0u8; mem::size_of::<u64>() * 2]);
        buf.0[7] = 1;
        buf.0[15] = 2;

        let block = MemoryReservationBlock::from_buffer(&buf.0).unwrap();
        assert_eq!(
            block.collect::<Vec<_>>(),
            vec![MemoryReservationEntry {
                address: 1,
                size: 2
            }]
        )
    }
}
