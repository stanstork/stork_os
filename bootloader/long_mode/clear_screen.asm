[bits 64]

; This function clears the VGA display by writing blank spaces to every character position.
; It expects the color/style to use in the rdi register.

clear_screen_64:
    ; Save the current state of registers rdi, rax, and rcx
    ; to avoid modifying their original values during the function.
    push rdi
    push rax
    push rcx

    ; Shift left the style information in rdi to prepare it
    ; for combining with the space character.
    ; This is because each VGA character cell consists of 
    ; the character and its style (like color).
    shl rdi, 8
    mov rax, rdi

    ; Set the lower byte of rax (al) to the space character.
    ; Now rax contains both the style (in the high byte)
    ; and the character (in the low byte).
    mov al, space_char

    ; Set rdi to point to the start of VGA memory.
    ; This is where the screen characters are stored.
    mov rdi, vga_start

    ; Set rcx to the number of VGA character cells.
    ; We divide the total size of VGA memory by 2 because
    ; each cell consists of 2 bytes: one for the character
    ; and one for its style.
    mov rcx, vga_extent / 2

    ; Use the 'rep stosw' instruction to write the value in rax
    ; (which contains the space character and style) into every
    ; character cell of VGA memory. This effectively clears the screen.
    rep stosw

    ; Restore the original values of rcx, rax, and rdi
    ; from the stack, as they were before entering the function.
    pop rcx
    pop rax
    pop rdi

    ret

space_char: equ ` `
