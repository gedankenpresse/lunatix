use crate::syscalls::{self, syscall};
use crate::Variant;

pub fn identify(cap: usize) -> Result<Variant, crate::Error> {
    let v = syscall(syscalls::SYS_IDENTIFY, cap, 0, 0, 0, 0, 0, 0)?;
    v.try_into()
}
