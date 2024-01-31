//! Handling of the DTB/FDT as a whole

use crate::fdt::structure::node::{NodeStructureError, StructureNode};
use crate::fdt::{
    FdtHeader, HeaderReadError, MemoryReservationBlock, MemoryReservationFormatError, Strings,
};
use thiserror_no_std::Error;

#[derive(Debug, Error, Eq, PartialEq)]
pub enum DtbError {
    #[error("Could not parse the fdt header: {0}")]
    HeaderParseError(#[from] HeaderReadError),
    #[error("Could not parse memory reservation block: {0}")]
    MemoryReservationError(#[from] MemoryReservationFormatError),
    #[error("Could not parse structure block: {0}")]
    StructureError(#[from] NodeStructureError),
}

pub struct DeviceTreeBlob<'buf> {
    pub header: FdtHeader,
    pub memory_reservations: MemoryReservationBlock<'buf>,
    pub structure: StructureNode<'buf>,
    pub strings: Strings<'buf>,
}

impl<'buf> DeviceTreeBlob<'buf> {
    pub fn from_buffer(buf: &'buf [u8]) -> Result<Self, DtbError> {
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
            strings,
            structure,
            memory_reservations: mem_resv_block,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use align_data::{include_aligned, Align64};

    #[test]
    fn parsing_qemu_sifive_u_works() {
        static DTB: &[u8] = include_aligned!(Align64, "../../test/data/qemu_sifive_u.dtb");
        let dtb = DeviceTreeBlob::from_buffer(DTB).unwrap();

        todo!();
    }

    #[test]
    fn parsing_qemu_virt_works() {
        static DTB: &[u8] = include_aligned!(Align64, "../../test/data/qemu_virt.dtb");
        let dtb = DeviceTreeBlob::from_buffer(DTB).unwrap();

        todo!();
    }
}
