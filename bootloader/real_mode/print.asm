[bits 16]

; Define a function to print a string to the screen using BIOS interrupts.
; The function expects a pointer to a null-terminated string in the bx register.

print_bios:
    ; Save the current state of registers ax and bx.
    ; This is important to preserve the context of the calling function.
    push ax
    push bx

    ; Set up ah for BIOS video service.
    ; ah = 0x0E is used for the teletype output function of BIOS interrupt 0x10.
    mov ah, 0x0E

    ; Start of the loop to print each character of the string.
    print_bios_loop:

        ; Check if the current character is the null terminator (end of string).
        cmp byte[bx], 0
        je print_bios_end  ; If it's null, jump to the end of the function.

        ; Load the current character from the string into al for printing.
        mov al, byte[bx]
        int 0x10  ; Call BIOS interrupt 0x10 to print the character in al.

        ; Increment the string pointer (bx) to point to the next character.
        inc bx

        ; Repeat the loop for the next character.
        jmp print_bios_loop

    ; Label for the end of the print function.
    print_bios_end:

    ; Restore the original state of bx and ax registers.
    pop bx
    pop ax

    ; Return to the calling function.
    ret
