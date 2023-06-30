.section .init
.globl _start
_start:
1:
	.option push
	.option norelax
	auipc ra, %pcrel_hi(1f)
	ld ra, %pcrel_lo(1b)(ra)
	jr ra
	.balign 8
1:	.dword _abs_start
	.option pop


.globl _abs_start
_abs_start:

	// Setup global pointer
	// comes from linker script
	.option push
	.option norelax
1:	auipc gp, %pcrel_hi(__global_pointer$)
	addi gp, gp, %pcrel_lo(1b)
	.option pop

	// put everything except hart0 to sleep
	// beqz a0, 2f
	// wfi
	// j 3f

2:
	// prepare hart0 stack
	// TODO: init other harts stacks in rust main
	.option push
	.option norelax
	// la sp, _stack_start
	.option pop

	//setup frame pointer
	add s0, sp, zero

	// continue in rust
	jal zero, _start_rust
3:



.globl wfi_spin
wfi_spin:
	wfi
	j wfi_spin
