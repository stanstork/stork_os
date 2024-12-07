OUTPUT_FORMAT("elf64-x86-64")
OUTPUT_ARCH(x86_64)

SECTIONS {
    . = 0x400000; /* Base virtual address of the ELF binary */

    /* .text starts at 0x401000 */
    .text : ALIGN(4K) {
        *(.text .text.*)
    }

    /* Read-only data section */
    .rodata : ALIGN(4K) {
        *(.rodata .rodata.*)
    }

    /* Data section: Contains initialized global variables */
    .data : ALIGN(4K) {
        *(.data .data.*)
    }

    /* BSS section: Uninitialized global variables */
    .bss : ALIGN(4K) {
        *(.bss .bss.*)
        *(COMMON)
    }

    /* Discard unnecessary sections */
    /DISCARD/ : {
        *(.note*)
        *(.comment)
        *(.eh_frame)
    }
}
