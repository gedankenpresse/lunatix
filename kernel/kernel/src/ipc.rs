use crate::caps;

#[repr(transparent)]
#[derive(Copy, Clone, Debug)]
pub struct Tag(usize);

pub type IpcResult = Result<usize, caps::Error>;

/// converts a Result for Ipc to register values.
/// The first argument is the error code. 0 means success
/// the second value is the return value in case of success
pub fn result_to_raw(res: IpcResult) -> (usize, usize) {
    match res {
        Ok(val) => (0, val),
        Err(e) => (e as usize, 0),
    }
}


impl Tag {
    #[inline(always)]
    pub const fn from_raw(raw: usize) -> Self {
        Tag(raw)
    }

    #[inline(always)]
    pub fn from_parts(label: usize, ncap: u8, nparam: u8) -> Tag {
        const LABELBITS: usize = core::mem::size_of::<usize>() * 8 - 16; 
        const LABELMASK: usize = (1 << LABELBITS) - 1;
        debug_assert_eq!(label & !LABELMASK, 0);
        return Self(label << 16 | (ncap as usize) << 8 | nparam as usize);
    }

    #[inline(always)]
    pub fn nparams(&self) -> u8 {
        (self.0 & ((1 << 8) - 1)) as u8
    }

    #[inline(always)]
    pub fn ncaps(&self) -> u8 {
        ((self.0 >> 8) & ((1 << 8) - 1)) as u8
    }

    #[inline(always)]
    pub fn label(&self) -> usize {
        self.0 >> 16
    }
}

pub struct RawMessage<'a> {
    pub cap_addresses: &'a [usize],
    pub params: &'a [usize],
}

impl RawMessage<'_> {
    pub fn from_args<'a>(tag: Tag, args: &'a [usize]) -> RawMessage<'a> {
        let caps = &args[..tag.ncaps() as usize];
        let rest = &args[tag.ncaps() as usize..];
        let params = &rest[..tag.nparams() as usize];
        return RawMessage { cap_addresses: caps, params: params };
    }
}
