/**
    This file describes the memory layout of the built ELF file.
 */

MEMORY {
    /* Contains the executable code */
    REGION_CODE (rx) : ORIGIN = 0x80200000, LENGTH = 256K

    /* Contains all static variables (bss + data) */
    REGION_STATIC : ORIGIN =    0x80300000, LENGTH = 64K
}
