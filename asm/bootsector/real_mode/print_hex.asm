[bits 16]

; Define a function to print a hexadecimal number using BIOS interrupts.
; The hexadecimal number to be printed is passed in register bx.

print_hex_bios:
    ; Save the current state of registers ax, bx, and cx.
    push ax
    push bx
    push cx

    ; Set up ah for BIOS video service.
    ; ah = 0x0E is the teletype output function of BIOS interrupt 0x10.
    mov ah, 0x0E

    ; Print the '0x' prefix for a hexadecimal number.
    mov al, '0'
    int 0x10  ; Print '0'.
    mov al, 'x'
    int 0x10  ; Print 'x'.

    ; Initialize cx as a counter for the number of hex digits.
    ; A 16-bit number has 4 hex digits (nibbles).
    mov cx, 4

    ; Start of the main loop to process and print each hex digit.
    print_hex_bios_loop:
        ; Check if all hex digits have been processed.
        cmp cx, 0
        je print_hex_bios_end

        ; Save the current value of bx.
        push bx

        ; Isolate the most significant nibble (4 bits) of bx.
        shr bx, 12

        ; Check if the nibble is >= 10, which corresponds to hexadecimal characters A-F.
        cmp bx, 10
        jge print_hex_bios_alpha

            ; If the nibble is less than 10, convert it to a character '0' to '9'.
            mov al, '0'
            add al, bl
            jmp print_hex_bios_loop_end

        print_hex_bios_alpha:
            ; If the nibble is >= 10, convert it to a character 'A' to 'F'.
            sub bl, 10  ; Adjust the value to the range 0-5.
            mov al, 'A'
            add al, bl

        print_hex_bios_loop_end:
            ; Print the current hexadecimal character.
            int 0x10

            ; Restore the original value of bx and prepare for the next nibble.
            pop bx
            shl bx, 4  ; Shift left to bring the next nibble to the most significant position.

            ; Decrement the counter.
            dec cx

            ; Repeat the loop for the next nibble.
            jmp print_hex_bios_loop

print_hex_bios_end:
    ; Restore the original state of registers cx, bx, and ax.
    pop cx
    pop bx
    pop ax

    ; Return to the calling function.
    ret
