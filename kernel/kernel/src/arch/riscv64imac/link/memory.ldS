/**
    This file describes the memory layout of the built ELF file.
 */

MEMORY {
    /* Contains the executable code */
    REGION_CODE (rx) : ORIGIN = 0xffffffff80000000, LENGTH = 16M
    /* Contains all static variables (bss + data) */
    REGION_STATIC : ORIGIN = 0xffffffff90000000, LENGTH = 16M
    /* Contains runtime managed variables and data (stack + heap) */
    REGION_RUNTIME : ORIGIN = 0xffffffffa0000000, LENGTH = 16M

/*    FLASH : ORIGIN = 0x80200000, LENGTH = 16M*/
/*    RAM : ORIGIN   = 0x81200000, LENGTH = 14M*/
/*    RSTACK : ORIGIN = 0x82000000, LENGTH = 2M*/
/*    RHEAP : ORIGIN  = 0x83000000, LENGTH = 16M*/
}

/*REGION_ALIAS("REGION_TEXT", FLASH);*/
/*REGION_ALIAS("REGION_RODATA", FLASH);*/
/*REGION_ALIAS("REGION_DATA", RAM);*/
/*REGION_ALIAS("REGION_BSS", RAM);*/
/*REGION_ALIAS("REGION_HEAP", RHEAP);*/
/*REGION_ALIAS("REGION_STACK", RSTACK);*/

/*_heap_size = 0x100000;*/
