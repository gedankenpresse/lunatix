//! Handling of the DTB/FDT as a whole

use crate::fdt::structure::node::{NodeStructureError, StructureNode};
use crate::fdt::{
    FdtHeader, HeaderReadError, MemoryReservationBlock, MemoryReservationFormatError, Strings,
};
use thiserror_no_std::Error;

/// The error that can occur when parsing a FDT
#[derive(Debug, Error, Eq, PartialEq)]
pub enum FdtError {
    /// The FDT header could not be parsed for a specific reason
    #[error("Could not parse the fdt header: {0}")]
    HeaderParseError(#[from] HeaderReadError),
    /// The memory reservation block could not be parsed for a specific reason
    #[error("Could not parse memory reservation block: {0}")]
    MemoryReservationError(#[from] MemoryReservationFormatError),
    /// The structure block could not be parsed for a specific reason
    #[error("Could not parse structure block: {0}")]
    StructureError(#[from] NodeStructureError),
}

/// A handle to a flattened device tree that has been parsed from an underlying buffer
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct FlattenedDeviceTree<'buf> {
    /// Metadata information about the device tree
    pub header: FdtHeader,
    /// Areas of the system memory which are reserved and should not be used without special care
    pub memory_reservations: MemoryReservationBlock<'buf>,
    /// Structure information about the device and its hardware
    pub structure: StructureNode<'buf>,
    /// The exact buffer that holds this device trees data
    pub buf: &'buf [u8],
}

impl<'buf> FlattenedDeviceTree<'buf> {
    /// Try to parse a FDT from a buffer
    pub fn from_buffer(buf: &'buf [u8]) -> Result<Self, FdtError> {
        let header = FdtHeader::read_from_buffer(buf)?;

        let mem_resv_block =
            MemoryReservationBlock::from_buffer(&buf[header.off_mem_rsvmap as usize..])?;
        let strings = Strings::from_buffer(
            &buf[header.off_dt_strings as usize
                ..header.off_dt_strings as usize + header.size_dt_strings as usize],
        );
        let structure = StructureNode::from_buffer_as_root(
            &buf[header.off_dt_struct as usize
                ..header.off_dt_struct as usize + header.size_dt_struct as usize],
            &strings,
        )?;

        Ok(Self {
            header,
            structure,
            memory_reservations: mem_resv_block,
            buf: &buf[0..header.total_size as usize],
        })
    }

    /// Try to read a FDT from a raw pointer
    ///
    /// # Safety
    /// The given pointer must be valid and the backing memory must be readable for at least 40 bytes after it.
    ///
    /// The underlying memory must also be valid for as long as the resulting instance is used.
    pub unsafe fn from_ptr(ptr: *const u8) -> Result<Self, FdtError> {
        let header = FdtHeader::from_ptr(ptr)?;
        let buf = core::slice::from_raw_parts::<u8>(ptr, header.total_size as usize);
        Self::from_buffer(buf)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use align_data::{include_aligned, Align64};
    extern crate std;

    #[test]
    fn parsing_qemu_sifive_u_works() {
        static DTB: &[u8] = include_aligned!(Align64, "../../test/data/qemu_sifive_u.dtb");
        let dtb = FlattenedDeviceTree::from_buffer(DTB).unwrap();

        assert_eq!(dtb.structure.name, "");
        assert_eq!(dtb.structure.children().nth(0).unwrap().name, "chosen");
        assert_eq!(dtb.structure.children().nth(1).unwrap().name, "aliases");
    }

    #[test]
    fn parsing_qemu_virt_works() {
        static DTB: &[u8] = include_aligned!(Align64, "../../test/data/qemu_virt.dtb");
        let dtb = FlattenedDeviceTree::from_buffer(DTB).unwrap();

        assert_eq!(dtb.structure.name, "");
        assert_eq!(dtb.structure.children().nth(0).unwrap().name, "poweroff");
        assert_eq!(dtb.structure.children().nth(1).unwrap().name, "reboot");
        assert_eq!(dtb.structure.props().count(), 4);
    }
}
