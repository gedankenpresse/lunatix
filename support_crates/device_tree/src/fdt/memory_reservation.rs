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
pub enum MemoryReservationFormatError {
    #[error("The memory reservation block is not aligned to an 8-byte boundary")]
    InvalidAlignment,
    #[error("The memory reservation block is smaller than 16 bytes")]
    BufferTooSmall,
    #[error("The memory reservation block does not contain a proper terminator")]
    NoTerminator,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MemoryReservationBlock<'buf> {
    buf: Option<&'buf [u8]>,
}

impl<'buf> MemoryReservationBlock<'buf> {
    pub fn from_buffer(buf: &'buf [u8]) -> Result<Self, MemoryReservationFormatError> {
        if buf.as_ptr() as usize % 8 != 0 {
            return Err(MemoryReservationFormatError::InvalidAlignment);
        }
        if buf.len() < mem::size_of::<u64>() * 2 {
            return Err(MemoryReservationFormatError::BufferTooSmall);
        }

        let num_entries = Self { buf: Some(buf) }.count() + 1;
        let block_size = mem::size_of::<u64>() * 2 * num_entries;
        // this .get() can only fail if the previous iterator consumed the whole buffer which means that no terminator was found earlier
        let buf = buf
            .get(0..block_size)
            .ok_or(MemoryReservationFormatError::NoTerminator)?;

        Ok(Self { buf: Some(buf) })
    }
}

impl<'buf> Iterator for MemoryReservationBlock<'buf> {
    type Item = MemoryReservationEntry;

    fn next(&mut self) -> Option<Self::Item> {
        let buf = self.buf?;

        // retrieve two u64 from the buffer
        let (addr, buf) = buf.split_at(mem::size_of::<u64>());
        let addr = u64::from_be_bytes(addr.try_into().unwrap());
        let (size, buf) = buf.split_at(mem::size_of::<u64>());
        let size = u64::from_be_bytes(size.try_into().unwrap());

        // if this entry is the specified terminator, finish iteration
        if addr == 0 && size == 0 {
            self.buf = None;
            return None;
        }
        // if the remaining buffer is too small, prevent further iteration in the future
        else if buf.len() < mem::size_of::<u64>() * 2 {
            self.buf = None;
        }
        // update remaining buffer
        else {
            self.buf = Some(buf);
        }

        Some(MemoryReservationEntry {
            address: addr,
            size,
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
        let mut buf = AlignedBuffer([0u8; 32]);
        buf.0[0..8].copy_from_slice(&1u64.to_be_bytes()); // addr = 1
        buf.0[8..16].copy_from_slice(&2u64.to_be_bytes()); // size = 2
        buf.0[16..24].fill(0); // terminator
        buf.0[24..32].fill(0); // terminator

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
