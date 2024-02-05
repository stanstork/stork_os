; Bootloader Assembly Code

; Set Program Origin
[org 0x7C00]

[bits 16] ; Start in 16-bit real mode

; Bootloader entry point.
bootsector_start:
    ; Initialize base pointer and stack pointer.
    ; This sets up a stack area for subroutine calls and local variables.
    mov bp, 0x0500
    mov sp, bp

    ; Save the boot drive ID provided by BIOS in dl to a memory location.
    mov byte[boot_drive], dl

    ; Print a welcome message using BIOS routines.
    mov bx, msg_hello_world
    call print_bios

    ; Load the next sector

    ; The first sector's already been loaded, so we start with the second sector
    ; of the drive. Note: Only bl will be used
    mov bx, 0x0002

    ; Now we want to load {n} sectors for the bootloader and kernel
    mov cx, 9

    ; Finally, we want to store the new sector immediately after the first
    ; loaded sector, at adress 0x7E00. This will help a lot with jumping between
    ; different sectors of the bootloader.
    mov dx, 0x7E00

    ; Now we're fine to load the new sectors
    call read_disk

    ; Transition to 32-bit protected mode.
    call enter_protected_mode

    ; Infinite loop to prevent CPU from executing beyond boot sector.
    jmp $

; Include external assembly files for various functionalities.
%include "real_mode/print.asm"
%include "real_mode/print_hex.asm"
%include "real_mode/read_disk.asm"
%include "real_mode/gdt_32.asm"
%include "real_mode/enter_pm.asm"

; Data storage and messages.
msg_hello_world: db `Hello World, from the BIOS!`, 0
boot_drive:      db 0x00

; Pad the boot sector to 510 bytes and add the 0xAA55 magic number.
times 510 - ($ - $$) db 0x00
dw 0xAA55

; Second sector starts here with 32-bit code.
[bits 32]

begin_protected:
    ; Clear the VGA screen in 32-bit mode.
    call clear_screen_32
    ; Check if the CPU supports 64-bit long mode.
    call detect_long_mode

    ; Test printing in protected mode.
    mov esi, protected_alert
    call print_string_32

    ; Initialize the page table for long mode.
    call init_page_table
    ; Enter 64-bit long mode.
    call enter_long_mode

    jmp $

; Include protected-mode function files.
%include "protected_mode/clear_screen.asm"
%include "protected_mode/print_string.asm"
%include "protected_mode/detect_lm.asm"
%include "protected_mode/init_pt.asm"
%include "protected_mode/gdt_64.asm"
%include "protected_mode/enter_lm.asm"

; Define constants and messages for protected mode.
vga_start:          equ 0x000B8000
vga_extent:         equ 80 * 25 * 2
style_wb:           equ 0x0F
protected_alert:    db `64-bit long mode supported`, 0

; Fill the rest of the sector with zeros.
times 512 - ($ - begin_protected) db 0x00

; Third sector starts here with 64-bit code.
[bits 64]

begin_long_mode:
    ; Clear screen and print a message in 64-bit long mode.
    mov rdi, style_blue
    call clear_screen_64
    mov rdi, style_blue
    mov rsi, long_mode_note
    call print_string_64

    ; Jump to the kernel start address.
    call kernel_start

    ; Infinite loop in 64-bit mode.
    jmp $

; Include long-mode function files.
%include "long_mode/clear_screen.asm"
%include "long_mode/print_string.asm"

; Define constants and messages for long mode.
kernel_start:       equ 0x8200
long_mode_note:     db `Now running in fully-enabled, 64-bit long mode!`, 0
style_blue:         equ 0x1F

; Fill the rest of the sector with zeros.
times 512 - ($ - begin_long_mode) db 0x00
