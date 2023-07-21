use libkernel::arch::trap::TrapFrame;

#[no_mangle]
pub fn handle_trap(tf: &mut TrapFrame) -> &mut TrapFrame {
    todo!();
}
