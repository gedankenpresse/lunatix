#[cfg(target_arch = "riscv64")]
mod riscv64;
mod userspace;

#[cfg(target_arch = "riscv64")]
pub(crate) use riscv64::*;

pub(crate) use userspace::*;
