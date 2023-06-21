INCLUDE memory.x

PROVIDE(_stext = ORIGIN(REGION_TEXT));
PROVIDE(_stack_start = ORIGIN(REGION_STACK) + LENGTH(REGION_STACK));
PROVIDE(_max_hart_id = 0);
PROVIDE(_hart_stack_size = 2K);
PROVIDE(_heap_start = ORIGIN(REGION_HEAP));
PROVIDE(_heap_size = 0);

PROVIDE(_mp_hook = default_mp_hook);
PROVIDE(__pre_init = default_pre_init);
PROVIDE(handle_trap = default_handle_trap);

SECTIONS
{
	.text.dummy (NOLOAD) :
	{
	/* This section is intended to make _stext address work */
	. = ABSOLUTE(_stext);
	} > REGION_TEXT

	.text _stext :
	{
	/* Put reset handler first in .text section so it ends up as the entry */
	/* point of the program. */
	KEEP(*(.init));
	KEEP(*(.init.rust));
	. = ALIGN(4);
	*(.trap);
	*(.trap.rust);

	*(.text .text.*);
	} > REGION_TEXT

	.rodata : ALIGN(4)
	{
	*(.srodata .srodata.*);
	*(.rodata .rodata.*);

	/* 4-byte align the end (VMA) of this section.
		This is required by LLD to ensure the LMA of the following .data
		section will have the correct alignment. */
	. = ALIGN(4);
	} > REGION_RODATA

	.data : ALIGN(4)
	{
	_sidata = LOADADDR(.data);
	_sdata = .;
	/* Must be called __global_pointer$ for linker relaxations to work. */
	PROVIDE(__global_pointer$ = . + 0x800);
	*(.sdata .sdata.* .sdata2 .sdata2.*);
	*(.data .data.*);
	. = ALIGN(4);
	_edata = .;
	} > REGION_DATA AT > REGION_RODATA

	.bss :
	{
		_sbss = .;
		*(.sbss .sbbs.* .bss .bss.*);
		. = ALIGN(4);
		_ebss = .;
	} > REGION_BSS

	.heap (NOLOAD) :
	{
		_sheap = .;
		. = ABSOLUTE(_heap_start);
		. += _heap_size;
		. = ALIGN(4);
		_eheap = .;
	} > REGION_HEAP


	/* fictitious region that represents the memory available for the stack */
  	.stack (NOLOAD) :
  	{
    	_estack = .;
    	. = ABSOLUTE(_stack_start);
    	_sstack = .;
  	} > REGION_STACK


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
