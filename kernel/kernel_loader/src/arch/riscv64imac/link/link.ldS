INCLUDE memory.ldS

/* provide heap and stack addresses as symbols because they are accessed in the kernel */
/* _heap_start = ORIGIN(REGION_RUNTIME); */
/* _stack_start = ORIGIN(REGION_RUNTIME) + LENGTH(REGION_RUNTIME); */

SECTIONS {
    /* empty region that fills the runtime region completely to prevent other sections from landing there */
    /* .runtime (NOLOAD) : {
        . = ORIGIN(REGION_RUNTIME) + LENGTH(REGION_RUNTIME);
    } > REGION_RUNTIME */

    .text : {
        *(.text .text.*);
    }

    .data : ALIGN(4) {
        *(.sdata .sdata.*);
        *(.data .data.*);
    } > REGION_STATIC

    /* put sections in appropriate memory regions where the linker fails to do so automatically */
    .ro_data : ALIGN(4096) {
        *(.rodata .rodata.* .eh_frame);
    } > REGION_STATIC
}
