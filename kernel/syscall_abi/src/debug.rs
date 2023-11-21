//! Definitions for the `debug_log` syscall.

use crate::{NoValue, RawSyscallArgs, SyscallBinding, SyscallResult};
use core::convert::Infallible;
use core::{mem, slice};

const USIZE2U8: usize = mem::size_of::<usize>() / mem::size_of::<u8>();

pub struct DebugLog;

#[derive(Debug, Eq, PartialEq)]
pub struct DebugLogArgs {
    /// How many bytes of the stored byte_slice contain the relevant string data.
    ///
    /// This is necessary because `byte_slice` is constant size and there is otherwise no way to known which parts of
    /// it are valid and which parts should be ignored.
    pub len: usize,

    /// A slice of bytes that is saved encoded into the CPUs registers and which contains UTF-8 string data
    pub byte_slice:
        [u8; (mem::size_of::<RawSyscallArgs>() / mem::size_of::<usize>() - 1) * USIZE2U8],
}

impl SyscallBinding for DebugLog {
    const SYSCALL_NO: usize = 0;
    type CallArgs = DebugLogArgs;
    type Return = SyscallResult<NoValue>;
}

impl TryFrom<RawSyscallArgs> for DebugLogArgs {
    type Error = Infallible;

    fn try_from(args: RawSyscallArgs) -> Result<Self, Self::Error> {
        let (len, reg_slice) = args.split_first().unwrap();
        let byte_slice: &[u8] = unsafe {
            slice::from_raw_parts(
                reg_slice.as_ptr().cast(),
                reg_slice.len() * (mem::size_of::<usize>() / mem::size_of::<u8>()),
            )
        };

        Ok(Self {
            len: *len,
            byte_slice: byte_slice.try_into().unwrap(),
        })
    }
}

impl Into<RawSyscallArgs> for DebugLogArgs {
    fn into(self) -> RawSyscallArgs {
        // reinterpret slice type
        let byte_slice = self.byte_slice.as_slice();
        let reg_slice: &[usize] = unsafe {
            slice::from_raw_parts(byte_slice.as_ptr().cast(), byte_slice.len() / USIZE2U8)
        };

        // construct a result
        let mut result: RawSyscallArgs = [0; 7];
        result[0] = self.len;
        result[1..].copy_from_slice(reg_slice);
        result
    }
}

#[cfg(test)]
mod test {
    use crate::debug::DebugLogArgs;
    use crate::RawSyscallArgs;
    use core::mem;

    #[test]
    fn test_args_are_correct_size() {
        assert_eq!(
            mem::size_of::<DebugLogArgs>(),
            mem::size_of::<RawSyscallArgs>()
        );
    }

    #[test]
    fn test_specific_to_raw_args() {
        // arrange
        let args = DebugLogArgs {
            len: 42,
            byte_slice: [0; 48],
        };

        // act
        let raw_args: RawSyscallArgs = args.into();

        // assert
        assert_eq!(raw_args, [42, 0, 0, 0, 0, 0, 0])
    }

    #[test]
    fn test_raw_to_specific_args() {
        // arrange
        let raw_args: RawSyscallArgs = [42, 0, 0, 0, 0, 0, 0];

        // act
        let args: DebugLogArgs = DebugLogArgs::try_from(raw_args).unwrap();

        // assert
        assert_eq!(
            args,
            DebugLogArgs {
                len: 42,
                byte_slice: [0; 48]
            }
        );
    }

    #[test]
    fn test_construct_from_string_slice() {
        // arrange
        let msg = "hi";

        // act
        let mut bytes = [0; 48];
        bytes[0..msg.len()].copy_from_slice(msg.as_bytes());
        let args = DebugLogArgs {
            len: msg.len(),
            byte_slice: bytes,
        };
    }
}

// Definitions for the `debug_putc` syscall
pub struct DebugPutc;

#[derive(Debug, Eq, PartialEq)]
pub struct DebugPutcArgs(pub char);

impl TryFrom<RawSyscallArgs> for DebugPutcArgs {
    type Error = Infallible;

    fn try_from(value: RawSyscallArgs) -> Result<Self, Self::Error> {
        Ok(Self(*value.first().unwrap() as u8 as char))
    }
}

impl Into<RawSyscallArgs> for DebugPutcArgs {
    fn into(self) -> RawSyscallArgs {
        [self.0 as usize, 0, 0, 0, 0, 0, 0]
    }
}

impl SyscallBinding for DebugPutc {
    const SYSCALL_NO: usize = 1;
    type CallArgs = DebugPutcArgs;
    type Return = SyscallResult<NoValue>;
}
