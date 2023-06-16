INCLUDE src/arch/riscv64imac/link/memory.x

PROVIDE(_stack_start = ORIGIN(REGION_STACK) + LENGTH(REGION_STACK));

PROVIDE(_mp_hook = default_mp_hook);
PROVIDE(__pre_init = default_pre_init);
PROVIDE(handle_trap = default_handle_trap);

SECTIONS
{
	/* this is the start of the text section */
	PROVIDE(_stext = ORIGIN(FLASH));

	.text ALIGN(_stext, 4) :
	{
		KEEP(*(.init));
		*(.text .text.*);
	} > FLASH

	.rodata ALIGN(4) :
	{
		*(.rodata .rodata.*);
	} > RAM


	.data : AT(LOADADDR(.rodata) + SIZEOF(.rodata))
	{
		_sidata = LOADADDR(.data);
		_sdata = .;
		PROVIDE(__global_pointer$ = . + 0x800);
		*(.data .data.*);
		. = ALIGN(4);
		_edata = .;
	} > RAM

	.bss :
	{
		_sbss = .;
		*(.sbss .sbbs.* .bss .bss.*);
		. = ALIGN(4);
		_ebss = .;
	} > RAM

	PROVIDE(_heap_size = 0);

	.heap (INFO) :
	{
		_sheap = .;
		. += _heap_size;
		. = ALIGN(4);
		_eheap = .;
	} > RAM


	/* fictitious region that represents the memory available for the stack */
  	.stack (NOLOAD) :
  	{
    	_estack = .;
    	. = ABSOLUTE(_stack_start);
    	_sstack = .;
  	} > REGION_STACK

	/*
	.stack _heap (INFO) : ALIGN(4)
	{
		_estack = .;
		. = _stack_start;
		_sstack = .;
	} > RAM
	*/

	/* fake output .got section */
  	/* Dynamic relocations are unsupported. This section is only used to detect
     relocatable code in the input files and raise an error if relocatable code
     is found */
	.got (INFO) :
	{
		KEEP *(.got .got.*);
	}

	.eh_frame (INFO) : { KEEP(*(.eh_frame)) }
	.eh_frame_hdr (INFO) : { *(.eh_frame_hdr) }
}


/* ASSERT(SIZEOF(.got) == 0, ".got section detected in the input files.
* dynamic relocation not supported. Compile C code without -fPIC flag.");
*/
