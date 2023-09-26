use crate::syscalls::syscall;
use syscall_abi::r#yield::{Yield, YieldArgs, YieldReturn};

pub fn r#yield() -> YieldReturn {
    syscall::<Yield>(YieldArgs {}).unwrap()
}
