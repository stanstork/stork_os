[bits 32]

; This function checks if the CPU supports Long Mode (64-bit),
; which is essential for running 64-bit code.

detect_long_mode:
    ; Save the current state of all general-purpose registers.
    pushad

    ; Check for CPUID support by toggling the ID bit in the EFLAGS register.
    ; First, copy the current EFLAGS to eax.
    pushfd                          ; Push FLAGS onto the stack.
    pop eax                         ; Pop it into eax for manipulation.

    ; Save the original FLAGS value in ecx for a comparison later.
    mov ecx, eax

    ; Toggle the ID bit (21st bit) in eax.
    xor eax, 1 << 21

    ; Write the modified value back to FLAGS.
    push eax
    popfd

    ; Read from FLAGS again to see if the ID bit can be toggled.
    pushfd
    pop eax

    ; Restore the original FLAGS value.
    push ecx
    popfd

    ; Compare the new FLAGS with the original.
    ; If they are equal, CPUID is not supported.
    cmp eax, ecx
    je cpuid_not_found_protected

    ; Check for extended CPUID functions.
    ; Load the highest extended function number into eax.
    mov eax, 0x80000000
    cpuid
    ; Compare if the CPU supports function 0x80000001.
    cmp eax, 0x80000001
    jb cpuid_not_found_protected

    ; Check for Long Mode support.
    mov eax, 0x80000001             ; Set CPUID argument for extended function.
    cpuid                           ; Execute CPUID with eax set to 0x80000001.
    test edx, 1 << 29               ; Test if Long Mode bit (29) is set in edx.
    jz lm_not_found_protected       ; If not set, Long Mode is not supported.

    ; If this point is reached, Long Mode is supported.
    ; Restore the registers and return.
    popad
    ret

cpuid_not_found_protected:
    call clear_screen_32            ; Clear the screen.
    mov esi, cpuid_not_found_str    ; Load the error message string.
    call print_string_32            ; Print the error message.
    jmp $                           ; Hang the system.

lm_not_found_protected:
    call clear_screen_32            ; Clear the screen.
    mov esi, lm_not_found_str       ; Load the error message string.
    call print_string_32            ; Print the error message.
    jmp $                           ; Hang the system.

lm_not_found_str:                   db `ERROR: Long mode not supported. Exiting...`, 0
cpuid_not_found_str:                db `ERROR: CPUID unsupported, but required for long mode`, 0
