use core::convert::Infallible;

use crate::{CAddr, NoValue, RawSyscallArgs, SyscallBinding, SyscallResult};

#[derive(Debug)]
pub struct MapDevmemArgs {
    pub devmem: CAddr,
    pub mem: CAddr,
    pub base: usize,
    pub len: usize,
}

pub struct MapDevmem;

impl SyscallBinding for MapDevmem {
    const SYSCALL_NO: usize = 17;
    type CallArgs = MapDevmemArgs;
    type Return = SyscallResult<NoValue>;
}

impl From<MapDevmemArgs> for RawSyscallArgs {
    fn from(args: MapDevmemArgs) -> Self {
        [args.devmem, args.mem, args.base, args.len, 0, 0, 0]
    }
}

impl TryFrom<RawSyscallArgs> for MapDevmemArgs {
    type Error = Infallible;

    fn try_from(args: RawSyscallArgs) -> Result<Self, Self::Error> {
        Ok(MapDevmemArgs {
            devmem: args[0],
            mem: args[1],
            base: args[2],
            len: args[3],
        })
    }
}
