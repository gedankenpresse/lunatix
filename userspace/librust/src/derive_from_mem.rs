use syscall_abi::CAddr;
use syscall_abi::derive_from_mem::{DeriveFromMem, DeriveFromMemArgs, DeriveFromMemReturn};
use syscall_abi::identify::CapabilityVariant;
use crate::syscalls::syscall;

pub fn derive_from_mem(src_mem: CAddr, target_slot: CAddr, target_cap: CapabilityVariant, size: Option<usize>) -> DeriveFromMemReturn {
    syscall::<DeriveFromMem>(DeriveFromMemArgs {
        src_mem,
        target_slot,
        target_cap,
        size,
    }).unwrap()
}