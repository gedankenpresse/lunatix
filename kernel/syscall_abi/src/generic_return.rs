use crate::RawSyscallReturn;

/// A generic return type that is used by syscalls which don't return anything specific.
#[derive(Debug, Eq, PartialEq)]
#[repr(usize)]
pub enum GenericReturn {
    Success = 0,
    UnsupportedSyscall = usize::MAX,
}

#[derive(Debug)]
pub struct UnidentifiableReturnCode;

impl From<GenericReturn> for RawSyscallReturn {
    fn from(value: GenericReturn) -> Self {
        [value as usize, 0]
    }
}

impl TryFrom<RawSyscallReturn> for GenericReturn {
    type Error = UnidentifiableReturnCode;

    fn try_from(value: RawSyscallReturn) -> Result<Self, Self::Error> {
        let reg0 = value[0];
        // TODO Figure out if this can be done in a more generic way
        match reg0 {
            0 => Ok(GenericReturn::Success),
            usize::MAX => Ok(GenericReturn::UnsupportedSyscall),
            _ => Err(UnidentifiableReturnCode),
        }
    }
}
