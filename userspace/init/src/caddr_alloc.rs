use core::sync::atomic::AtomicUsize;

use librust::syscall_abi::CAddr;

static CADDR_ALLOC: AtomicUsize = AtomicUsize::new(10);

pub fn alloc_caddr() -> CAddr {
    let addr = CADDR_ALLOC.fetch_add(1, core::sync::atomic::Ordering::SeqCst);
    assert!(addr < 64);
    return addr;
}
