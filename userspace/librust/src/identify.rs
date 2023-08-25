use crate::syscalls::syscall;
use syscall_abi::identify::{Identify, IdentifyArgs, IdentifyReturn};

pub fn identify(caddr: usize) -> IdentifyReturn {
    syscall::<Identify>(IdentifyArgs { caddr }).unwrap()
}
