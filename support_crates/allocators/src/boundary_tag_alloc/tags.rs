use core::mem;

// TODO Implement multiple tag types with different sized size fields (u16, u32, u64, usize)

#[derive(Debug, Eq, PartialEq)]
#[repr(u8)]
pub(super) enum AllocationState {
    // TODO Rename to AllocationMarker
    Free = 1,
    Allocated = 2,
}

/// The tag which is placed at the beginning of a chunk of memory and stores details about that chunk.
#[derive(Debug, Eq, PartialEq)]
#[repr(C)]
pub(super) struct BeginTag {
    pub block_size: u8, // TODO Rename to content_size
    pub state: AllocationState,
}

impl BeginTag {
    pub fn as_bytes(&self) -> &[u8; 2] {
        unsafe { mem::transmute(self) }
    }

    pub fn from_bytes(value: &[u8]) -> Self {
        assert_eq!(
            value.len(),
            2,
            "BeginTag can only be reconstructed from 2-byte long slices"
        );
        assert!(
            value[1] == AllocationState::Free as u8 || value[1] == AllocationState::Allocated as u8,
            "value has invalid allocation tag"
        );
        Self {
            block_size: value[0],
            state: unsafe { mem::transmute(value[1]) },
        }
    }
}

/// The tag which is placed at the end of an allocated memory area.
#[derive(Debug, Eq, PartialEq)]
#[repr(C)]
pub(super) struct EndTag {
    pub block_size: u8,
}

impl EndTag {
    pub fn as_bytes(&self) -> &u8 {
        unsafe { mem::transmute(self) }
    }

    pub fn from_bytes(value: &u8) -> &Self {
        unsafe { mem::transmute(value) }
    }
}
