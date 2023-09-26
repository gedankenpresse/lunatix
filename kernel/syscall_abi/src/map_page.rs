//! Definitions for the `map_page` syscall.

use crate::generic_return::{GenericReturn, UnidentifiableReturnCode};
use crate::{CAddr, RawSyscallArgs, RawSyscallReturn, SyscallBinding};
use bitflags::bitflags;
use core::convert::Infallible;

pub struct MapPage;

bitflags! {
    #[derive(Debug, Eq, PartialEq, Default)]
    pub struct MapPageFlag: usize {
        /// The page should be mapped so that it is readable.
        const READ = 0b001;
        /// The page should be mapped so that it is writable.
        const WRITE = 0b010;
        /// The page should be mapped so that code stored in it can be executed.
        const EXEC = 0b100;
    }
}

#[derive(Debug, Eq, PartialEq)]
#[repr(C)]
pub struct MapPageArgs {
    /// The page capability that should be mapped
    pub page: CAddr,
    /// The vspace into which the page should be mapped
    pub vspace: CAddr,
    /// The memory capability from which intermediate page tables can be allocated
    pub mem: CAddr,
    /// The memory address at which the page should be mapped
    pub addr: usize,
    /// The flags with which the page should be mapped
    pub flags: MapPageFlag,
}

#[derive(Debug, Eq, PartialEq)]
#[repr(usize)]
pub enum MapPageReturn {
    Success = 0,
    InvalidPageCAddr = 1,
    InvalidVSpaceCAddr = 2,
    InvalidMemCAddr = 3,
    NoMem = 4,
    UnsupportedSyscall = usize::MAX,
}

impl SyscallBinding for MapPage {
    const SYSCALL_NO: usize = 5;
    type CallArgs = MapPageArgs;
    type Return = MapPageReturn;
}

impl From<MapPageArgs> for RawSyscallArgs {
    fn from(args: MapPageArgs) -> Self {
        [
            args.page,
            args.vspace,
            args.mem,
            args.addr,
            args.flags.bits(),
            0,
            0,
        ]
    }
}

impl TryFrom<RawSyscallArgs> for MapPageArgs {
    type Error = Infallible;

    fn try_from(args: RawSyscallArgs) -> Result<Self, Self::Error> {
        Ok(Self {
            page: args[0],
            vspace: args[1],
            mem: args[2],
            addr: args[3],
            flags: MapPageFlag::from_bits_truncate(args[4]),
        })
    }
}

impl Into<GenericReturn> for MapPageReturn {
    fn into(self) -> GenericReturn {
        match self {
            MapPageReturn::Success => GenericReturn::Success,
            MapPageReturn::InvalidPageCAddr
            | MapPageReturn::InvalidVSpaceCAddr
            | MapPageReturn::InvalidMemCAddr
            | MapPageReturn::NoMem => GenericReturn::Error,
            MapPageReturn::UnsupportedSyscall => GenericReturn::UnsupportedSyscall,
        }
    }
}

impl Into<RawSyscallReturn> for MapPageReturn {
    fn into(self) -> RawSyscallReturn {
        match self {
            MapPageReturn::Success => [0, 0],
            MapPageReturn::InvalidPageCAddr => [1, 0],
            MapPageReturn::InvalidVSpaceCAddr => [2, 0],
            MapPageReturn::InvalidMemCAddr => [3, 0],
            MapPageReturn::NoMem => [4, 0],
            MapPageReturn::UnsupportedSyscall => [usize::MAX, 0],
        }
    }
}

impl TryFrom<RawSyscallReturn> for MapPageReturn {
    type Error = UnidentifiableReturnCode;

    fn try_from(value: RawSyscallReturn) -> Result<Self, Self::Error> {
        let discriminant = value[0];
        match discriminant {
            0 => Ok(MapPageReturn::Success),
            1 => Ok(MapPageReturn::InvalidPageCAddr),
            2 => Ok(MapPageReturn::InvalidVSpaceCAddr),
            3 => Ok(MapPageReturn::InvalidMemCAddr),
            4 => Ok(MapPageReturn::NoMem),
            usize::MAX => Ok(MapPageReturn::UnsupportedSyscall),
            _ => Err(UnidentifiableReturnCode),
        }
    }
}
