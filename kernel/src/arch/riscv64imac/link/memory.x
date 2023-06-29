MEMORY

{
    FLASH : ORIGIN = 0x80200000, LENGTH = 2M
    RAM : ORIGIN   = 0x80400000, LENGTH = 1M
    RSTACK: ORIGIN = 0x80500000, LENGTH = 1M
    RHEAP: ORIGIN  = 0x80600000, LENGTH = 16M
}

REGION_ALIAS("REGION_TEXT", FLASH);
REGION_ALIAS("REGION_RODATA", FLASH);
REGION_ALIAS("REGION_DATA", RAM);
REGION_ALIAS("REGION_BSS", RAM);
REGION_ALIAS("REGION_HEAP", RHEAP);
REGION_ALIAS("REGION_STACK", RSTACK);

_heap_size = 0x100000;
