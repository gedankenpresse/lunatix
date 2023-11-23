use crate::{CAddr, IpcTag, RawSyscallArgs, SyscallBinding, SyscallResult};

pub const NUM_DATA_REGS: usize = 5;

pub struct Receive;

impl SyscallBinding for Receive {
    const SYSCALL_NO: usize = 24;
    type CallArgs = ReceiveArgs;
    type Return = SyscallResult<ReceiveReturn>;
}

#[derive(Eq, PartialEq, Debug)]
pub struct ReceiveArgs {
    pub target: CAddr,
    pub tag: IpcTag,
}

impl From<RawSyscallArgs> for ReceiveArgs {
    fn from(value: RawSyscallArgs) -> Self {
        Self {
            target: value[0].into(),
            tag: IpcTag::from_raw(value[1]),
        }
    }
}

impl From<ReceiveArgs> for RawSyscallArgs {
    fn from(value: ReceiveArgs) -> Self {
        [value.target.into(), value.tag.into(), 0, 0, 0, 0, 0]
    }
}

#[derive(Debug)]
pub struct ReceiveReturn {
    pub tag: IpcTag,
    pub raw_args: [usize; NUM_DATA_REGS],
}

impl From<RawSyscallArgs> for ReceiveReturn {
    fn from(value: RawSyscallArgs) -> Self {
        let [tag, a0, a1, a2, a3, a4, _] = value;
        Self {
            tag: IpcTag::from_raw(tag),
            raw_args: [a0, a1, a2, a3, a4],
        }
    }
}

impl Into<RawSyscallArgs> for ReceiveReturn {
    fn into(self) -> RawSyscallArgs {
        let [a0, a1, a2, a3, a4] = self.raw_args;
        [self.tag.as_raw(), a0, a1, a2, a3, a4, 0]
    }
}
