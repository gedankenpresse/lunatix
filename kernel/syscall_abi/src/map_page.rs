//! Definitions for the `map_page` syscall.

use crate::generic_return::{GenericReturn, UnidentifiableReturnCode};
use crate::{CAddr, NoValue, RawSyscallArgs, RawSyscallReturn, SyscallBinding, SyscallResult};
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

impl SyscallBinding for MapPage {
    const SYSCALL_NO: usize = 5;
    type CallArgs = MapPageArgs;
    type Return = SyscallResult<NoValue>;
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
