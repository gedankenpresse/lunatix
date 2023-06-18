// Utilities which are sometimes called from rust for functionality that is just easier to
// implement in assemby

.option arch, +zicsr    // enable supervisor mode assembly extensions

.text

.option push
.option norelax
.balign 4

// s scratch
.global read_sscratch
read_sscratch:
	csrr a0,sscratch
	ret

.global write_sscratch
write_sscratch:
	csrw sscratch, a0
	ret

.global read_write_sscratch
read_write_sscratch:
	csrrw a0, sscratch, a0
	ret


// s status
.global read_set_sstatus
read_set_sstatus:
	csrrs a0, sstatus, a0
	ret

.global read_clear_sstatus
read_clear_sstatus:
	csrrc a0, sstatus, a0
	ret

.global read_sstatus
read_sstatus:
	csrr a0, sstatus
	ret

// s epc
.global write_sepc
write_sepc:
	csrw sepc, a0
	ret

.global read_sepc
read_sepc:
	csrr a0, sepc
	ret

// s ie
.global read_sie
read_sie:
	csrr a0, sie
	ret

.global set_sie
set_sie:
	csrs sie, a0
	ret

.global clear_sie
clear_sie:
	csrc sie, a0
	ret

// s ip
.global read_sip
read_sip:
	csrr a0, sip
	ret

.global set_sip
set_sip:
	csrs sip, a0
	ret

.global clear_sip
clear_sip:
	csrc sip, a0
	ret



.global read_stvec
read_stvec:
	csrr a0, stvec
	ret

.global write_stvec
write_stvec:
	csrw stvec, a0
	ret

.global read_write_stvec
read_write_stvec:
	csrrw a0, stvec, a0
	ret

.global read_mstatus
read_mstatus:
	csrr a0, mstatus
	ret

.global wait_for_interrupt
wait_for_interrupt:
	wfi
	ret
.option pop
