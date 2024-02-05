use core::mem;
use thiserror_no_std::Error;

const HEADER_MAGIC: u32 = 0xd00dfeed;

/// Errors that can occur when reading the FDT header
#[derive(Debug, Error, Eq, PartialEq)]
pub enum HeaderReadError {
    /// The provided buffer did not contain the required magic bytes at the start
    #[error("The provided buffer did not contain the required magic bytes at the start")]
    InvalidMagic,
    /// The provided buffer did not contain enough bytes to read a header from it
    #[error("The provided buffer did not contain enough bytes to read a header from it")]
    BufferTooSmall,
    /// The device tree blob is encoded using an unsupported version
    #[error("The device tree blob is encoded using version {0} (with last compatible version being {1}) which is not supported")]
    UnsupportedVersion(u32, u32),
    /// The device tree blob is not aligned to an 8-byte boundary
    #[error("The device tree blob is not aligned to an 8-byte boundary")]
    InvalidAlignment,
}

/// The FDT-Header data structure present at the start of every device tree blob.
/// All the header fields are 32-bit integers, stored in big-endian format.
///
/// It is modelled according to the [Spec Section 5.2](https://devicetree-specification.readthedocs.io/en/latest/chapter5-flattened-format.html#header).
#[derive(Debug, Eq, PartialEq)]
#[repr(C)]
pub struct FdtHeader {
    /// This field shall contain the value 0xd00dfeed (big-endian).
    pub magic: u32,
    /// This field shall contain the total size in bytes of the devicetree data structure.
    /// This size shall encompass all sections of the structure: the header, the memory reservation block, structure block and strings block, as well as any free space gaps between the blocks or after the final block.
    pub total_size: u32,
    /// This field shall contain the offset in bytes of the structure block (see [Spec Section 5.4](https://devicetree-specification.readthedocs.io/en/latest/chapter5-flattened-format.html#sect-fdt-structure-block)) from the beginning of the header.
    pub off_dt_struct: u32,
    /// This field shall contain the offset in bytes of the strings block (see [Spec Section 5.5](https://devicetree-specification.readthedocs.io/en/latest/chapter5-flattened-format.html#sect-fdt-strings-block)) from the beginning of the header.
    pub off_dt_strings: u32,
    /// This field shall contain the offset in bytes of the memory reservation block (see [Spec Section 5.3](https://devicetree-specification.readthedocs.io/en/latest/chapter5-flattened-format.html#sect-fdt-memory-reservation-block)) from the beginning of the header.
    pub off_mem_rsvmap: u32,
    /// This field shall contain the version of the devicetree data structure.
    /// The version is `17` if using the structure as supported by this library.
    /// An DTSpec boot program may provide the devicetree of a later version, in which case this field shall contain the version number defined in whichever later document gives the details of that version.
    pub version: u32,
    /// This field shall contain the lowest version of the devicetree data structure with which the version used is backwards compatible.
    /// So, for the structure as supported by this library (version `17`), this field shall contain `16` because version `17` is backwards compatible with version `16`, but not earlier versions.
    /// As per [Spec Section 5.1](https://devicetree-specification.readthedocs.io/en/latest/chapter5-flattened-format.html#sect-fdt-versioning), a DTSpec boot program should provide a devicetree in a format which is backwards compatible with version `16`, and thus this field shall always contain `16`.
    pub last_comp_version: u32,
    /// This field shall contain the physical ID of the system’s boot CPU.
    /// It shall be identical to the physical ID given in the reg property of that CPU node within the devicetree.
    pub boot_cpuid_phys: u32,
    /// This field shall contain the length in bytes of the strings block section of the devicetree blob.
    pub size_dt_strings: u32,
    /// This field shall contain the length in bytes of the structure block section of the devicetree blob.
    pub size_dt_struct: u32,
}

impl FdtHeader {
    /// Try to read a header from a provided buffer
    pub fn read_from_buffer(buf: &[u8]) -> Result<Self, HeaderReadError> {
        // check alignment
        if (buf.as_ptr() as usize) % 8 != 0 {
            return Err(HeaderReadError::InvalidAlignment);
        }

        fn read_u32(buf: &[u8]) -> Result<(u32, &[u8]), HeaderReadError> {
            if buf.len() < mem::size_of::<u32>() {
                return Err(HeaderReadError::BufferTooSmall);
            }

            let (head, tail) = buf.split_at(mem::size_of::<u32>());
            let value = u32::from_be_bytes(head.try_into().unwrap());
            Ok((value, tail))
        }

        let (magic, buf) = read_u32(buf)?;
        if magic != HEADER_MAGIC {
            return Err(HeaderReadError::InvalidMagic);
        }
        let (total_size, buf) = read_u32(buf)?;
        let (off_dt_struct, buf) = read_u32(buf)?;
        let (off_dt_strings, buf) = read_u32(buf)?;
        let (off_mem_rsvmap, buf) = read_u32(buf)?;
        let (version, buf) = read_u32(buf)?;
        let (last_comp_version, buf) = read_u32(buf)?;
        if last_comp_version != 16 {
            return Err(HeaderReadError::UnsupportedVersion(
                version,
                last_comp_version,
            ));
        }
        let (boot_cpuid_phys, buf) = read_u32(buf)?;
        let (size_dt_strings, buf) = read_u32(buf)?;
        let (size_dt_struct, _buf) = read_u32(buf)?;

        Ok(Self {
            magic,
            total_size,
            off_dt_struct,
            off_dt_strings,
            off_mem_rsvmap,
            version,
            last_comp_version,
            boot_cpuid_phys,
            size_dt_strings,
            size_dt_struct,
        })
    }

    /// Try to read a header from a provided memory location
    ///
    /// # Safety
    /// The given pointer must be valid and the backing memory must be readable for at least 40 bytes after it.
    pub unsafe fn from_ptr(ptr: *const u8) -> Result<Self, HeaderReadError> {
        let buf = core::slice::from_raw_parts::<u8>(ptr, mem::size_of::<FdtHeader>());
        Self::read_from_buffer(buf)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[repr(C, align(8))]
    pub struct AlignedBuffer<const LENGTH: usize>(pub [u8; LENGTH]);

    #[test]
    fn read_from_buffer_fails_if_buffer_too_small() {
        let buf = AlignedBuffer([0u8; 2]);
        assert_eq!(
            FdtHeader::read_from_buffer(&buf.0),
            Err(HeaderReadError::BufferTooSmall)
        );
    }

    #[test]
    fn read_from_buffer_fails_with_invalid_magic_bytes() {
        let buf = AlignedBuffer([0u8; mem::size_of::<FdtHeader>()]);
        assert_eq!(
            FdtHeader::read_from_buffer(&buf.0),
            Err(HeaderReadError::InvalidMagic)
        )
    }

    #[test]
    fn read_from_buffer_fails_with_invalid_version() {
        let mut buf = AlignedBuffer([0u8; mem::size_of::<FdtHeader>()]);
        // header
        buf.0[0] = 0xd0;
        buf.0[1] = 0x0d;
        buf.0[2] = 0xfe;
        buf.0[3] = 0xed;
        // version
        buf.0[5 * 4 + 3] = 0x02;
        // last compatible version
        buf.0[6 * 4 + 3] = 0x01;
        assert_eq!(
            FdtHeader::read_from_buffer(&buf.0),
            Err(HeaderReadError::UnsupportedVersion(0x02, 0x01)),
        )
    }

    #[test]
    fn read_from_buffer_succeeds() {
        let mut buf = AlignedBuffer([0u8; mem::size_of::<FdtHeader>()]);
        // header
        buf.0[0] = 0xd0;
        buf.0[1] = 0x0d;
        buf.0[2] = 0xfe;
        buf.0[3] = 0xed;
        // version
        buf.0[5 * 4 + 3] = 17;
        // last compatible version
        buf.0[6 * 4 + 3] = 16;
        assert_eq!(
            FdtHeader::read_from_buffer(&buf.0),
            Ok(FdtHeader {
                magic: HEADER_MAGIC,
                total_size: 0,
                off_dt_struct: 0,
                off_dt_strings: 0,
                off_mem_rsvmap: 0,
                version: 17,
                last_comp_version: 16,
                boot_cpuid_phys: 0,
                size_dt_strings: 0,
                size_dt_struct: 0,
            }),
        )
    }

    #[test]
    fn read_from_ptr() {
        let mut buf = AlignedBuffer([0u8; mem::size_of::<FdtHeader>()]);
        // header
        buf.0[0] = 0xd0;
        buf.0[1] = 0x0d;
        buf.0[2] = 0xfe;
        buf.0[3] = 0xed;
        // version
        buf.0[5 * 4 + 3] = 17;
        // last compatible version
        buf.0[6 * 4 + 3] = 16;

        let ptr = &buf.0 as *const u8;
        let read_header = unsafe { FdtHeader::from_ptr(ptr) };
        assert_eq!(
            read_header,
            Ok(FdtHeader {
                magic: HEADER_MAGIC,
                total_size: 0,
                off_dt_struct: 0,
                off_dt_strings: 0,
                off_mem_rsvmap: 0,
                version: 17,
                last_comp_version: 16,
                boot_cpuid_phys: 0,
                size_dt_strings: 0,
                size_dt_struct: 0,
            })
        );
    }
}
