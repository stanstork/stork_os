[bits 64]

; This function displays a string on the screen using the VGA memory area.
; It requires the style (like color) to be passed in rdi and the address
; of the message string in rsi.

print_string_64:
    ; Save the state of the registers rax, rdx, rdi, and rsi
    ; to preserve their values throughout this function.
    push rax
    push rdx
    push rdi
    push rsi

    ; Initialize rdx to point to the start of VGA memory.
    ; This is where the text will be displayed on screen.
    mov rdx, vga_start

    ; Shift the style data in rdi left by 8 bits to prepare it for
    ; combination with the character data in rax.
    shl rdi, 8

    ; Start of the main loop to print each character.
    print_string_64_loop:
        ; Check if the current character is a null terminator.
        ; If it is, the string is complete, and we exit the loop.
        cmp byte[rsi], 0
        je  print_string_64_done

        ; Check if the current position has reached the end of VGA memory.
        ; If it has, we cannot print more and exit the loop.
        cmp rdx, vga_start + vga_extent
        je print_string_64_done

        ; Move the style data into the high byte of rax and the character
        ; to be printed into the low byte of rax.
        mov rax, rdi
        mov al, byte[rsi]

        ; Write the character and its style to the current VGA memory location.
        mov word[rdx], ax

        ; Move to the next character in the string and the next position
        ; in VGA memory (noting that each VGA cell is 2 bytes).
        add rsi, 1
        add rdx, 2

        ; Jump back to the start of the loop to print the next character.
        jmp print_string_64_loop

print_string_64_done:
    ; Restore the original values of rsi, rdi, rdx, and rax
    ; from the stack, as they were before entering the function.
    pop rsi
    pop rdi
    pop rdx
    pop rax

    ret
