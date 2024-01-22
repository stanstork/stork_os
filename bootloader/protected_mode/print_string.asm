[bits 32]

; This function provides a simple print routine in 32-bit protected mode.
; It directly writes characters to VGA memory, bypassing BIOS utilities.

print_string_32:
    ; Save the state of all general-purpose registers.
    ; This ensures that we can use them freely without affecting
    ; the state of the calling function.
    pushad

    ; Set edx to point to the start of VGA memory.
    ; VGA memory is a specific area in memory used for text display.
    mov edx, vga_start

    ; Start of the main loop to print each character.
    print_string_32_loop:
        ; Check if the current character is a null terminator.
        ; If it is, the string is complete, and we exit the loop.
        cmp byte[esi], 0
        je  print_string_32_done

        ; Load the character to be printed into al.
        ; The style (like color) is loaded into ah.
        mov al, byte[esi]        ; Load the character from the message.
        mov ah, style_wb         ; Load the predefined style (e.g., white on black).

        ; Combine the character and style and write them to the VGA memory.
        mov word[edx], ax

        ; Increment esi to point to the next character in the string.
        ; Increment edx to move to the next position in VGA memory.
        ; Note: Each position in VGA memory is 2 bytes (character + style).
        add esi, 1
        add edx, 2

        ; Jump back to the start of the loop to process the next character.
        jmp print_string_32_loop

print_string_32_done:
    ; Restore the original state of all general-purpose registers
    ; and return from the function.
    popad
    ret
