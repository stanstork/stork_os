[bits 16]

; This function reads sectors from a disk into memory using BIOS interrupts.
; Inputs: Sector start point in bx, Number of sectors to read in cx, Destination address in dx.

read_disk:
    ; Save the registers
    push ax
    push bx
    push cx
    push dx

    ; Save the number of registers to load for later
    push cx

    ; For the ATA Read bios utility, the value of ah must be 0x02
    mov ah, 0x02

    ; Move the number of sectors to read into al (BIOS interrupt expects it in al).
    mov al, cl

    ; Move the sector start point into cl (BIOS interrupt expects it in cl).
    mov cl, bl

    ; Move the destination address into bx (BIOS interrupt expects it in bx).
    mov bx, dx

    mov ch, 0x00        ; Cylinder goes in ch
    mov dh, 0x00        ; Cylinder head goes in dh

    ; Store boot drive in dl
    mov dl, byte[boot_drive]

    ; Perform the BIOS disk read
    int 0x13

    ; Check read error
    jc bios_disk_error

    ; Pop number of sectors to read
    ; Compare with sectors actually read
    pop bx
    cmp al, bl
    jne bios_disk_error

    ; If all goes well, we can now print the success message and return
    mov bx, success_msg
    call print_bios

    ; Restore the registers
    pop dx
    pop cx
    pop bx
    pop ax

    ; Return
    ret


bios_disk_error:
    ; Print out the error code and hang, since
    ; the program didn't work correctly
    mov bx, error_msg
    call print_bios

    ; The error code is in ah, so shift it down to mask out al
    shr ax, 8
    mov bx, ax
    call print_hex_bios

    ; Infinite loop to hang
    jmp $

error_msg:              db `ERROR Loading Sectors. Code: `, 0
success_msg:            db `nAdditional Sectors Loaded Successfully!`, 0