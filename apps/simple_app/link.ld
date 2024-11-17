OUTPUT_FORMAT(elf64-x86-64)
ENTRY(_start)

SECTIONS {
    .text : {
        *(.text*)      /* Include all `.text` sections */
    }

    .rodata : {
        *(.rodata*)    /* Include all `.rodata` sections */
    }

    .data : {
        *(.data*)      /* Include all `.data` sections */
    }

    .bss : {
        *(.bss*)       /* Include all `.bss` sections */
        . = ALIGN(4);  /* Align the end of the `.bss` section */
    }
}