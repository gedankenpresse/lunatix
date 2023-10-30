use crate::syscalls::syscall;
use syscall_abi::exit::Exit;

pub fn exit() -> ! {
    syscall::<Exit>(Default::default()).unwrap();
    unreachable!();
}
