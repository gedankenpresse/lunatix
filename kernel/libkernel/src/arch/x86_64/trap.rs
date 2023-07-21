#[repr(C)]
#[derive(Debug)]
pub struct TrapFrame {}

impl TrapFrame {
    pub fn null() -> Self {
        todo!()
    }

    pub fn set_stack_start(&mut self, stack_start: usize) {
        todo!()
    }

    pub fn set_entry_point(&mut self, entry_point: usize) {
        todo!()
    }

    pub fn get_ipc_args(&mut self) -> &[usize] {
        todo!()
    }

    pub fn write_syscall_result(&mut self, a0: usize, a1: usize) {
        todo!()
    }
}

pub fn enable_interrupts() {
    todo!()
}
