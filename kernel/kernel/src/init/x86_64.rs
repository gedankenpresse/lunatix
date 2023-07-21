use allocators::Arena;
use libkernel::mem::{MemoryPage, PageTable};

pub(crate) fn init_kernel_pagetable() -> &'static mut PageTable {
    todo!();
}

pub(crate) fn init_trap_handler_stack(allocator: &mut Arena<'static, MemoryPage>) -> *mut () {
    todo!();
}

pub(crate) fn init_kernel_trap_handler(
    allocator: &mut Arena<'static, MemoryPage>,
    trap_stack_start: *mut (),
) {
    todo!();
}

pub(crate) fn run_init(trap_stack: *mut ()) {
    todo!();
}
