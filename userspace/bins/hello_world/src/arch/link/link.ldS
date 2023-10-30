/* this linker script ensures that all sections tagged as LOAD are page-aligned */

SECTIONS {
    .text : ALIGN(4096) {
        *(.text .text.*);
    }

    .data : ALIGN(4096) {
        *(.sdata .sdata.*);
        *(.data .data.*);
    }

    .ro_data : ALIGN(4096) {
        *(.rodata .rodata.* .eh_frame);
    }
}
