// Utilities which are sometimes called from rust for functionality that is just easier to
// implement in assemby

.option arch, +zicsr    // enable supervisor mode assembly extensions

.text

.option push
.option norelax
.balign 4

.global read_mstatus
read_mstatus:
	csrr a0, mstatus
	ret

.global wait_for_interrupt
wait_for_interrupt:
	wfi
	ret
.option pop
