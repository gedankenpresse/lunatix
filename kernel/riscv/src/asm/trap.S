// a register is 8 bytes wide
# define REGBYTES 8

.option arch, +zicsr    // enable supervisor mode assembly extensions

.text                   // emit this in the programs code section

// disable instruction generation using the global pointer
// also needed to prevent an unsupported R_RISCV_ALIGN relocation from being generated
.option push
.option norelax

// logically, the trap handler should be at least aligned on a
// 4 byte boundary, but the .align derective doesn't seem to be
// doing byte alignments
.align 4

// define this routine as a symbol that can be externally linked to
.globl asm_trap_handler
asm_trap_handler:
	// save registers using sscratch as tmp value and the bottom register (t6) to store the TrapFrame pointer
	// (which was originally saved in sscratch by rust when the trapped frame was started)

	csrrw t6, sscratch, t6      // atomically swap t6 and sscratch content

	// save registers (with corresponding offsets to the buffer pointed to by t6)
	// these values will land in the `general_purpose_regs` and `floating_point_regs` fields in TrapFrame
	sd x0, 0*REGBYTES(t6)
	sd x1, 1*REGBYTES(t6)
	sd x2, 2*REGBYTES(t6)
	sd x3, 3*REGBYTES(t6)
	sd x4, 4*REGBYTES(t6)
	sd x5, 5*REGBYTES(t6)
	sd x6, 6*REGBYTES(t6)
	sd x7, 7*REGBYTES(t6)
	sd x8, 8*REGBYTES(t6)
	sd x9, 9*REGBYTES(t6)
	sd x10, 10*REGBYTES(t6)
	sd x11, 11*REGBYTES(t6)
	sd x12, 12*REGBYTES(t6)
	sd x13, 13*REGBYTES(t6)
	sd x14, 14*REGBYTES(t6)
	sd x15, 15*REGBYTES(t6)
	sd x16, 16*REGBYTES(t6)
	sd x17, 17*REGBYTES(t6)
	sd x18, 18*REGBYTES(t6)
	sd x19, 19*REGBYTES(t6)
	sd x20, 20*REGBYTES(t6)
	sd x21, 21*REGBYTES(t6)
	sd x22, 22*REGBYTES(t6)
	sd x23, 23*REGBYTES(t6)
	sd x24, 24*REGBYTES(t6)
	sd x25, 25*REGBYTES(t6)
	sd x26, 26*REGBYTES(t6)
	sd x27, 27*REGBYTES(t6)
	sd x28, 28*REGBYTES(t6)
	sd x29, 29*REGBYTES(t6)
	sd x30, 30*REGBYTES(t6)

	// floating point registers are currently ignored

    // t6 was originally swapped with sscratch so we have to swap it back and save it too
	mv a0, t6			        // copy trap frame pointer to a0
	csrrw t6, sscratch, t6 	    // restore original t6 value
	sd x31, (31*REGBYTES)(a0)   // store actual t6/x31 register the same way the other registers were

	// call rust trap handler with some arguments (a0 to a7 registers)
	ld sp, (64*REGBYTES)(a0)    // load rust trap handler stack pointer (third field in TrapFrame)

	// load kernel saved registers
	ld s0, (0*REGBYTES)(sp)     // save kernel saved registers
	ld s1, (1*REGBYTES)(sp)     // save kernel saved registers
	ld s2, (2*REGBYTES)(sp)     // save kernel saved registers
	ld s3, (3*REGBYTES)(sp)     // save kernel saved registers
	ld s4, (4*REGBYTES)(sp)     // save kernel saved registers
	ld s5, (5*REGBYTES)(sp)     // save kernel saved registers
	ld s6, (6*REGBYTES)(sp)     // save kernel saved registers
	ld s7, (7*REGBYTES)(sp)     // save kernel saved registers
	ld s8, (8*REGBYTES)(sp)     // save kernel saved registers
	ld s9, (9*REGBYTES)(sp)     // save kernel saved registers
	ld s10, (10*REGBYTES)(sp)    // save kernel saved registers
	ld s11, (11*REGBYTES)(sp)    // save kernel saved registers
	ld ra, (12*REGBYTES)(sp)     // save kernel return address
	ld gp, (13*REGBYTES)(sp)     // save kernel global pointer
	ld tp, (14*REGBYTES)(sp)     // save kernel thread pointer

	addi sp,sp,(15*REGBYTES)   // deallocate stack space for kernel registers
	// return to kernel
	ret
.option pop


// a0 ist the trap frame pointer which holds the tasks PC in its
.globl trap_frame_load
trap_frame_load:
	addi sp,sp,(-15*REGBYTES)   // allocate stack space for kernel registers
	sd s0, (0*REGBYTES)(sp)     // save kernel saved registers
	sd s1, (1*REGBYTES)(sp)     // save kernel saved registers
	sd s2, (2*REGBYTES)(sp)     // save kernel saved registers
	sd s3, (3*REGBYTES)(sp)     // save kernel saved registers
	sd s4, (4*REGBYTES)(sp)     // save kernel saved registers
	sd s5, (5*REGBYTES)(sp)     // save kernel saved registers
	sd s6, (6*REGBYTES)(sp)     // save kernel saved registers
	sd s7, (7*REGBYTES)(sp)     // save kernel saved registers
	sd s8, (8*REGBYTES)(sp)     // save kernel saved registers
	sd s9, (9*REGBYTES)(sp)     // save kernel saved registers
	sd s10, (10*REGBYTES)(sp)    // save kernel saved registers
	sd s11, (11*REGBYTES)(sp)    // save kernel saved registers
	sd ra, (12*REGBYTES)(sp)     // save kernel return address
	sd gp, (13*REGBYTES)(sp)     // save kernel global pointer
	sd tp, (14*REGBYTES)(sp)     // save kernel thread pointer

	sd sp, (64*REGBYTES)(a0)    // store current stack pointer (third field in TrapFrame)


    ld t6, (65*REGBYTES)(a0)    // load program counter (4th field in TrapFrame struct) into t6
	csrw sepc, t6   	        // configure cpu to return to that program counter upon sret
	mv t6, a0 			        // move trap frame pointer to t6
	csrw sscratch, t6 	        // store trap frame pointer to sscratch

	// restore registers of the frame
	// this modifies the bottom register last, so that t6 doesn't have to be saved anyhere
	ld x0, 0*REGBYTES(t6)
	ld x1, 1*REGBYTES(t6)
	ld x2, 2*REGBYTES(t6)
	ld x3, 3*REGBYTES(t6)
	ld x4, 4*REGBYTES(t6)
	ld x5, 5*REGBYTES(t6)
	ld x6, 6*REGBYTES(t6)
	ld x7, 7*REGBYTES(t6)
	ld x8, 8*REGBYTES(t6)
	ld x9, 9*REGBYTES(t6)
	ld x10, 10*REGBYTES(t6)
	ld x11, 11*REGBYTES(t6)
	ld x12, 12*REGBYTES(t6)
	ld x13, 13*REGBYTES(t6)
	ld x14, 14*REGBYTES(t6)
	ld x15, 15*REGBYTES(t6)
	ld x16, 16*REGBYTES(t6)
	ld x17, 17*REGBYTES(t6)
	ld x18, 18*REGBYTES(t6)
	ld x19, 19*REGBYTES(t6)
	ld x20, 20*REGBYTES(t6)
	ld x21, 21*REGBYTES(t6)
	ld x22, 22*REGBYTES(t6)
	ld x23, 23*REGBYTES(t6)
	ld x24, 24*REGBYTES(t6)
	ld x25, 25*REGBYTES(t6)
	ld x26, 26*REGBYTES(t6)
	ld x27, 27*REGBYTES(t6)
	ld x28, 28*REGBYTES(t6)
	ld x29, 29*REGBYTES(t6)
	ld x30, 30*REGBYTES(t6)
	ld x31, 31*REGBYTES(t6)         // this overwrites t6

	// floating point registers are currently ignored

	sret                            // return from trap handler
