/**
    This file describes the memory layout of the built ELF file.
 */

MEMORY {
    /* Contains the executable code */
    /* Minimum required size for debug builds, as of now 108K */
    REGION_CODE (rx) : ORIGIN = 0xffffffff00000000, LENGTH = 133K

    /* Contains all static variables (bss + data) */
    /* Minimum required size for debug bilds, as of now 57K, without init included */
    REGION_STATIC : ORIGIN = 0xffffffff10000000, LENGTH = 80K
}
