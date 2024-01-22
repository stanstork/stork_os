; Define the Flat Mode Configuration Global Descriptor Table (GDT) for 64-bit mode.
; The GDT in flat mode allows unrestricted access to read and write code anywhere in memory.

align 4  ; Align the GDT on a 4-byte boundary for optimal access speed.

gdt_64_start:  ; Start of the GDT definition.

; Define the null descriptor for the 64-bit GDT.
; The null descriptor is a mandatory entry that should be placed at the start of the GDT.
gdt_64_null:
    dd 0x00000000           ; Set the first 32 bits to 0.
    dd 0x00000000           ; Set the second 32 bits to 0.

; Define the code segment descriptor for the 64-bit GDT.
; This descriptor is used for the code segment in long mode.
gdt_64_code:
    ; Base address is 0, limit is 4GB (0xFFFFF) to span the entire memory.
    dw 0xFFFF           ; Limit (bits 0-15) - defines the size of the segment.
    dw 0x0000           ; Base  (bits 0-15) - lower part of the segment base address.
    db 0x00             ; Base  (bits 16-23) - middle part of the segment base address.

    ; Access and flag bits.
    db 0b10011010       ; Access byte: present, ring 0, code segment, readable, not accessed.
    db 0b10101111       ; Flags: granularity (1), 64-bit mode, limit (bits 16-19).
    db 0x00             ; Base  (bits 24-31) - upper part of the segment base address.

; Define the data segment descriptor for the 64-bit GDT.
; This descriptor is used for the data segment in long mode.
gdt_64_data:
    ; Base address is 0, limit is 0 to define a flat data segment.
    dw 0x0000           ; Limit (bits 0-15) - defines the size of the segment.
    dw 0x0000           ; Base  (bits 0-15) - lower part of the segment base address.
    db 0x00             ; Base  (bits 16-23) - middle part of the segment base address.

    ; Access and flag bits.
    db 0b10010010       ; Access byte: present, ring 0, data segment, writable, not accessed.
    db 0b10100000       ; Flags: granularity (1), 64-bit mode, limit (bits 16-19).
    db 0x00             ; Base  (bits 24-31) - upper part of the segment base address.

gdt_64_end:  ; End of the GDT definition.

; Define the GDT descriptor.
; This structure provides the CPU with the size and starting address of the GDT.
gdt_64_descriptor:
    dw gdt_64_end - gdt_64_start - 1   ; Size of the GDT (subtract 1 for correct count).
    dd gdt_64_start                    ; Start address of the 64-bit GDT.

; Define helpers to find pointers to the Code and Data segments.
code_seg_64: equ gdt_64_code - gdt_64_start  ; Offset for the code segment descriptor.
data_seg_64: equ gdt_64_data - gdt_64_start  ; Offset for the data segment descriptor.
