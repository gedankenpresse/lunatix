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
    type CallArgs: TryFrom<RawSyscallArgs> + Into<RawSyscallArgs>;

    /// The type that is used to encode the syscalls result.
    ///
    /// The syscall result is usually specific to a syscall but must be a superset of `GenericReturn` which is why
    /// conversion to and from `GenericReturn` must be possible.
    type Return: FromRawSysResponse + IntoRawSysRepsonse;
}

/// A trait binding a syscall to a `repr(C)` type which is expected to be put into the tasks IPC buffer when calling it.
pub trait IpcArgsBinding: SyscallBinding {
    type IpcArgs;
}

/// A trait binding a syscall to a `repr(C)` type which the kernel puts into the tasks IPC buffer as a result when
/// called.
pub trait IpcReturnBinding: SyscallBinding {
    type IpcReturn;
}

/// The arguments to a syscall as they are encoded in the CPUs registers.
pub type RawSyscallArgs = [usize; 7];

/// The return value of a syscall as they are encoded in the CPUs registers.
pub type RawSyscallReturn = [usize; 2];

#[derive(Debug, Copy, Clone)]
pub struct NoValue;

impl TryFrom<usize> for NoValue {
    type Error = ();

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        if value == 0 {
            Ok(NoValue)
        } else {
            Err(())
        }
    }
}

impl Into<usize> for NoValue {
    fn into(self) -> usize {
        0
    }
}

pub trait FromRawSysResponse {
    fn from_response(raw: RawSyscallReturn) -> Self;
}

pub trait IntoRawSysRepsonse {
    fn into_response(self) -> RawSyscallReturn;
}

pub type SyscallResult<T> = Result<T, Error>;

impl<T> IntoRawSysRepsonse for SyscallResult<T>
where
    T: Into<usize>,
{
    fn into_response(self) -> RawSyscallReturn {
        match self {
            Ok(v) => [0, v.into()],
            Err(e) => [e as usize, 0],
        }
    }
}

impl<T> FromRawSysResponse for Result<T, Error>
where
    T: TryFrom<usize>,
{
    fn from_response(raw: RawSyscallReturn) -> Self {
        match raw {
            [0, v] => match T::try_from(v) {
                Ok(v) => Ok(v),
                Err(_) => Err(Error::ValueInvalid),
            },
            [e, _] => match Error::try_from(e) {
                Ok(e) => Err(e),
                Err(_) => Err(Error::UnknownError),
            },
        }
    }
}

use crate::Error;
use bitflags::bitflags;

bitflags! {
    #[derive(Copy, Clone, Debug, Eq, PartialEq, Default)]
    pub struct MapFlags: usize {
        /// The page should be mapped so that it is readable.
        const READ = 0b001;
        /// The page should be mapped so that it is writable.
        const WRITE = 0b010;
        /// The page should be mapped so that code stored in it can be executed.
        const EXEC = 0b100;
    }
}
