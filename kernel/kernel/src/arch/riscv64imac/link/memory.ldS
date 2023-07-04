/**
    This file describes the memory layout of the built ELF file.
 */

MEMORY {
    /* Contains the executable code */
    REGION_CODE (rx) : ORIGIN = 0xffffffff00000000, LENGTH = 16M
    /* Contains all static variables (bss + data) */
    REGION_STATIC : ORIGIN = 0xffffffff10000000, LENGTH = 16M
}
