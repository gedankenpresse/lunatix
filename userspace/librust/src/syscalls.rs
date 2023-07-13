use core::arch::asm;

const SYS_DEBUG_LOG: usize = 0;
const SYS_DEBUG_PUTC: usize = 1;

#[inline(always)]
fn syscall(
    syscallno: usize,
    a1: usize,
    a2: usize,
    a3: usize,
    a4: usize,
    a5: usize,
    a6: usize,
    a7: usize,
) -> (usize, usize) {
    let mut out0: usize;
    let mut out1: usize;
    unsafe { asm!(
        "ecall",
        inout("x10") syscallno => out0,
        inout("x11") a1 => out1,
        in("x12") a2,
        in("x13") a3,
        in("x14") a4,
        in("x15") a5,
        in("x16") a6,
        in("x17") a7,
    ); }
    return (out0, out1);
}

pub(crate) fn syscall_writeslice(s: &[u8]) {
    const REG_SIZE: usize = core::mem::size_of::<usize>();
    let mut reg_buf: [usize; 6] = [0usize; 6];
    unsafe { 
        let buf: &mut [u8] = core::slice::from_raw_parts_mut(reg_buf.as_mut_ptr().cast(), REG_SIZE * reg_buf.len());
        assert!(s.len() <= buf.len());
        buf[..s.len()].clone_from_slice(s);
    }
    let [a2, a3, a4, a5, a6, a7] = reg_buf;
    syscall(SYS_DEBUG_LOG, s.len(), a2, a3, a4, a5, a6, a7);
}

pub (crate) fn syscall_putc(c: u8) {
    unsafe { asm!("ecall", in("x10") SYS_DEBUG_PUTC, in("x11") c) }
}