use core::arch::asm;

pub(crate) const SYS_DEBUG_LOG: usize = 0;
pub(crate) const SYS_DEBUG_PUTC: usize = 1;
pub(crate) const SYS_SEND: usize = 2;
pub(crate) const SYS_IDENTIFY: usize = 3;

#[inline(always)]
pub(crate) fn raw_syscall(
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
    unsafe {
        asm!(
            "ecall",
            inout("x10") syscallno => out0,
            inout("x11") a1 => out1,
            in("x12") a2,
            in("x13") a3,
            in("x14") a4,
            in("x15") a5,
            in("x16") a6,
            in("x17") a7,
        );
    }
    return (out0, out1);
}

#[inline(always)]
pub(crate) fn syscall(
    syscallno: usize,
    a1: usize,
    a2: usize,
    a3: usize,
    a4: usize,
    a5: usize,
    a6: usize,
    a7: usize,
) -> Result<usize, crate::Error> {
    let (a0, a1) = raw_syscall(syscallno, a1, a2, a3, a4, a5, a6, a7);
    if a0 == 0 {
        return Ok(a1);
    }
    return Err(a0.into());
}
