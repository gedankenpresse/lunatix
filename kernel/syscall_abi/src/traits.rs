use crate::errors::SyscallError;
use core::fmt::Debug;

/// A trait for binding a syscall number to its specific argument and return type.
pub trait SyscallBinding {
    /// The syscall number which identifies this syscall.
    const SYSCALL_NO: usize;

    /// The type that is used to encode the syscalls arguments.
    ///
    /// These are usually syscall specific but are required to be represent themselves as `RawSyscallArgs` since that
    /// is what is written to the CPUs registers when the syscall is executed.
    /// Accordingly, the kernel needs to be able to reconstruct the arguments by reading the registers and thus,
    /// a backwards conversion from `RawSyscallArgs` must also be possible.
    type CallArgs: TryFrom<RawSyscallArgs> + Into<RawSyscallArgs> + Debug;

    /// The type that is used to encode the syscalls result.
    ///
    /// The syscall result is usually specific to a syscall but must be a superset of `GenericReturn` which is why
    /// conversion to and from `GenericReturn` must be possible.
    type Return: FromRawSysResponse + IntoRawSysRepsonse + Debug;
}

/// The arguments to a syscall as they are encoded in the CPUs registers.
pub type RawSyscallArgs = [usize; 7];

/// The return value of a syscall as they are encoded in the CPUs registers.
pub type RawSyscallReturn = [usize; 8];

/// The data that is returned on a successful syscall invocation
pub type SyscallReturnData = [usize; 7];

/// A type that is used when a syscall requires no arguments or returns nothing.
#[derive(Debug, Copy, Clone)]
pub struct NoValue;

impl From<SyscallReturnData> for NoValue {
    fn from(_value: SyscallReturnData) -> Self {
        NoValue
    }
}

impl Into<SyscallReturnData> for NoValue {
    fn into(self) -> SyscallReturnData {
        [0, 0, 0, 0, 0, 0, 0]
    }
}

pub trait FromRawSysResponse {
    fn from_response(raw: RawSyscallReturn) -> Self;
}

pub trait IntoRawSysRepsonse {
    fn into_response(self) -> RawSyscallReturn;
}

pub type SyscallResult<T> = Result<T, SyscallError>;

impl<T> IntoRawSysRepsonse for SyscallResult<T>
where
    T: Into<SyscallReturnData>,
{
    fn into_response(self) -> RawSyscallReturn {
        match self {
            Ok(v) => {
                let inner = v.into();
                [
                    0, inner[0], inner[1], inner[2], inner[3], inner[4], inner[5], inner[6],
                ]
            }
            Err(e) => [e as usize, 0, 0, 0, 0, 0, 0, 0],
        }
    }
}

impl<T> FromRawSysResponse for SyscallResult<T>
where
    T: TryFrom<SyscallReturnData>,
{
    fn from_response(raw: RawSyscallReturn) -> Self {
        match raw {
            [0, v1, v2, v3, v4, v5, v6, v7] => match T::try_from([v1, v2, v3, v4, v5, v6, v7]) {
                Ok(v) => Ok(v),
                Err(_) => Err(SyscallError::ValueInvalid),
            },
            [e, ..] => match SyscallError::try_from(e) {
                Ok(e) => Err(e),
                Err(_) => Err(SyscallError::UnknownError),
            },
        }
    }
}
