[bits 32]

; This function clears the VGA display by writing blank spaces to every character position.
; It operates in 32-bit protected mode and takes no arguments.

clear_screen_32:
    ; Save the current state of all general-purpose registers.
    ; This ensures that we can use them freely without affecting
    ; the state of the calling function.
    pushad

    ; Set up constants for clearing.
    ; ebx will hold the size of the VGA memory area.
    ; ecx will point to the start of VGA memory.
    ; edx will be used as a counter.
    mov ebx, vga_extent
    mov ecx, vga_start
    mov edx, 0

    ; Start of the main loop to clear the screen.
    clear_screen_32_loop:
        ; Check if the counter (edx) has reached the end of the VGA memory area.
        ; If it has, we are done clearing.
        cmp edx, ebx
        jge clear_screen_32_done

        ; Temporarily save the value of edx on the stack.
        push edx

        ; Set al to the space character and ah to the white-on-black style.
        ; This will make ax contain both the character and the style.
        mov al, space_char
        mov ah, style_wb

        ; Calculate the address to write to by adding edx (offset) to ecx (base address).
        ; Then write the character and style to the VGA memory.
        add edx, ecx
        mov word[edx], ax

        ; Restore the original value of edx from the stack.
        pop edx

        ; Increment the counter by 2, since each VGA entry is 2 bytes.
        add edx, 2

        ; Jump back to the start of the loop to process the next character.
        jmp clear_screen_32_loop

clear_screen_32_done:
    ; Restore the original state of all general-purpose registers
    ; and return from the function.
    popad
    ret

space_char: equ ` `
