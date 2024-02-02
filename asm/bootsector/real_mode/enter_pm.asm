[bits 16]

; This function transitions the CPU from 16-bit real mode to 32-bit protected mode.

enter_protected_mode:
    ; Disable interrupts.
    ; This is necessary because the transition to 32-bit mode can cause
    ; interrupt handlers (which are still 16-bit) to behave unpredictably.
    cli

    ; Load the Global Descriptor Table (GDT) for 32-bit mode.
    ; The GDT defines various memory segments and their properties.
    lgdt [gdt_32_descriptor]

    ; Enable Protected Mode.
    ; This is done by setting the Protection Enable (PE) bit of the Control Register 0 (CR0).
    ; We first move CR0 to a general-purpose register (eax), modify it, and then write it back.
    mov eax, cr0
    or eax, 0x00000001  ; Set the PE bit in CR0.
    mov cr0, eax

    ; Clear the pipeline with a far jump to flush out any residual 16-bit instructions.
    ; This jump is to a 32-bit segment, effectively starting 32-bit execution.
    jmp code_seg:init_pm

    [bits 32]
    init_pm:
    ; We are now in 32-bit mode.

    ; Set up the segment registers with the flat mode data segment selector.
    ; This step is essential because the old 16-bit segment selectors are no longer valid.
    mov ax, data_seg
    mov ds, ax
    mov ss, ax
    mov es, ax
    mov fs, ax
    mov gs, ax

    ; Initialize the stack pointers.
    ; This is crucial as the previous stack was in 16-bit real mode, and we need a new 32-bit stack.
    mov ebp, 0x90000    ; Set base pointer for stack.
    mov esp, ebp        ; Set stack pointer.

    ; Jump to the 32-bit code segment to continue execution in protected mode.
    jmp begin_protected
