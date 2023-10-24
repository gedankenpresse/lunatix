#![no_std]

use core::sync::atomic::Ordering::SeqCst;
use core::{cell::UnsafeCell, sync::atomic::AtomicUsize};
use librust::syscall_abi::CAddr;

pub trait CAddressAllocator {
    fn alloc_caddr(&self) -> CAddr;
}

unsafe impl Send for GlobalCaddrAllocator {}
unsafe impl Sync for GlobalCaddrAllocator {}
pub struct GlobalCaddrAllocator {
    cell: UnsafeCell<Option<&'static dyn CAddressAllocator>>,
}

impl GlobalCaddrAllocator {
    const fn new() -> Self {
        Self {
            cell: UnsafeCell::new(None),
        }
    }
}

pub static CADDR_ALLOC: GlobalCaddrAllocator = GlobalCaddrAllocator::new();

pub unsafe fn set_global_caddr_allocator(alloc: &'static dyn CAddressAllocator) {
    let inner = CADDR_ALLOC.cell.get().as_mut().unwrap();
    assert!(inner.is_none());
    let _ = inner.insert(alloc);
}

pub fn alloc_caddr() -> CAddr {
    let inner = unsafe { CADDR_ALLOC.cell.get().as_ref().unwrap().unwrap() };
    inner.alloc_caddr()
}

pub struct CAddrAlloc {
    pub max: AtomicUsize,
    pub cur: AtomicUsize,
}

impl CAddressAllocator for CAddrAlloc {
    fn alloc_caddr(&self) -> CAddr {
        let addr = self.cur.fetch_add(1, SeqCst);
        assert!(addr < self.max.load(SeqCst));
        return addr;
    }
}
