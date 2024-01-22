[bits 32]

; This function transitions the processor from 32-bit protected mode to 64-bit long mode.

enter_long_mode:
    ; Enable the no-execute bit.
    ; Set the 9th bit (bit index 8) of the IA32_EFER MSR (Model-Specific Register) to 1.
    ; This is necessary to enter long mode.
    mov ecx, 0xC0000080        ; IA32_EFER MSR ID.
    rdmsr                      ; Read the current value of the IA32_EFER MSR into edx:eax.
    or eax, 1 << 8             ; Set the 9th bit to 1 (enables no-execute bit).
    wrmsr                      ; Write the modified value back to the IA32_EFER MSR.

    ; Enable paging.
    ; Set the 31st bit of the control register CR0.
    ; This is required to enable protected mode and paging.
    mov eax, cr0               ; Move the current value of CR0 into eax.
    or eax, 1 << 31            ; Set the PG (paging) bit (31st bit) to 1.
    mov cr0, eax               ; Write the modified value back to CR0.

    ; Load the Global Descriptor Table for 64-bit mode.
    ; The GDT defines the characteristics of the various memory areas used during operation.
    lgdt [gdt_64_descriptor]   ; Load the 64-bit GDT.

    ; Perform a far jump to the code segment defined for 64-bit mode.
    ; This is necessary to flush the pipeline and properly switch to 64-bit mode.
    jmp code_seg_64:init_lm    ; Jump to the 64-bit code segment.

[bits 64]
init_lm:
    ; Initialize segment registers in 64-bit mode.
    cli                        ; Clear the interrupt flag to disable interrupts.
    mov ax, data_seg_64        ; Load the 64-bit data segment selector into ax.
    mov ds, ax                 ; Set the data segment to the value in ax.
    mov es, ax                 ; Set the extra segment to the value in ax.
    mov fs, ax                 ; Set the F-segment to the value in ax.
    mov gs, ax                 ; Set the G-segment to the value in ax.
    mov ss, ax                 ; Set the stack segment to the value in ax.

    ; Jump to the main 64-bit mode code.
    ; This is where the execution will continue in 64-bit mode.
    jmp begin_long_mode        ; Jump to the 64-bit code labeled 'begin_long_mode'.
