INCLUDE memory.ldS

/* provide heap and stack addresses as symbols because they are accessed in the kernel */
/* this is not needed anymore, because the kernel_loader initializes heap/stack */
/* _heap_start = ORIGIN(REGION_RUNTIME); */
/* _stack_start = ORIGIN(REGION_RUNTIME) + LENGTH(REGION_RUNTIME); */

SECTIONS {

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
