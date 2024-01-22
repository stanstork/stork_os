[bits 16]

; Define the Flat Mode Configuration Global Descriptor Table (GDT)
; The flat mode GDT allows unrestricted access to read and write code anywhere in memory.
; This setup is for 32-bit protected mode.

gdt_32_start:

; Define the null descriptor for the 32-bit GDT.
; A null descriptor is mandatory as the first entry in the GDT for memory integrity checks.
gdt_32_null:
    dd 0x00000000           ; Set all 32 bits to 0 for the null descriptor.
    dd 0x00000000           ; Set the next 32 bits to 0, completing the null descriptor.

; Define the code segment descriptor for the 32-bit GDT.
; This segment is used for executing code in protected mode.
gdt_32_code:
    ; Descriptor details:
    ; Base: 0x00000, Limit: 0xFFFFF (4 GB), giving the segment full access to memory.
    ; Access byte: present (1), privilege level (00), descriptor type (1), executable (1), conforming (0), readable (1), accessed (0).
    ; Flags: granularity (1), 32-bit default (1), 64-bit segment (0), AVL (0).

    dw 0xFFFF           ; Limit low (bits 0-15)
    dw 0x0000           ; Base low (bits 0-15)
    db 0x00             ; Base middle (bits 16-23)
    db 0b10011010       ; Access byte
    db 0b11001111       ; Flags and Limit high (bits 16-19)
    db 0x00             ; Base high (bits 24-31)

; Define the data segment descriptor for the 32-bit GDT.
; This segment is used for data storage in protected mode.
gdt_32_data:
    ; Descriptor details similar to the code segment, 
    ; but with the executable bit cleared indicating a data segment.

    dw 0xFFFF           ; Limit low (bits 0-15)
    dw 0x0000           ; Base low (bits 0-15)
    db 0x00             ; Base middle (bits 16-23)
    db 0b10010010       ; Access byte
    db 0b11001111       ; Flags and Limit high (bits 16-19)
    db 0x00             ; Base high (bits 24-31)

gdt_32_end:

; Define the GDT descriptor.
; This structure provides the CPU with the size and starting address of the GDT.
gdt_32_descriptor:
    dw gdt_32_end - gdt_32_start - 1  ; Size of GDT, one byte less than true size.
    dd gdt_32_start                   ; Start address of the 32-bit GDT.

; Define helpers for offset calculations.
; These help to easily find the offsets of the code and data segments within the GDT.
code_seg: equ gdt_32_code - gdt_32_start
data_seg: equ gdt_32_data - gdt_32_start
