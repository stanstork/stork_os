[bits 32]

; This function initializes the page table for 64-bit long mode.
; It sets up paging structures including PML4T, PDPT, PDT, and PT,
; and maps the lowest 2MB of physical memory into virtual memory.

init_page_table:
    ; Save the state of all general-purpose registers.
    pushad

    ; Clear a memory area of size 4096 bytes for page table structures.
    ; We start at address 0x1000 and clear the area using 'rep stosd'.
    mov edi, 0x1000              ; Start of the page table structure.
    mov cr3, edi                 ; Set cr3 to the base of our page tables.
    xor eax, eax                 ; Zero out eax to clear memory.
    mov ecx, 4096                ; Number of double words to clear.
    rep stosd                    ; Clear memory.

    ; Set up the first entry of each table in the hierarchy: PML4T, PDPT, PDT.
    ; We set up each entry to point to the next level table with appropriate flags.
    mov edi, cr3                 ; Reset edi to the base of PML4T.

    ; Set up the first entry of PML4T.
    mov dword[edi], 0x2003       ; Point PML4T[0] to PDPT at 0x2000 with flags 0x3 (present, writable).
    add edi, 0x1000              ; Move edi to the base of PDPT.

    ; Set up the first entry of PDPT.
    mov dword[edi], 0x3003       ; Point PDPT[0] to PDT at 0x3000 with flags 0x3.
    add edi, 0x1000              ; Move edi to the base of PDT.

    ; Set up the first entry of PDT.
    mov dword[edi], 0x4003       ; Point PDT[0] to PT at 0x4000 with flags 0x3.

    ; Fill in the final Page Table (PT).
    ; Map each entry in PT to a 4KB physical memory block,
    ; starting from 0x0000 to 0x200000 (2MB).
    add edi, 0x1000              ; Move edi to the base of PT.
    mov ebx, 0x00000003          ; EBX holds the address 0x0000 with flags 0x3.
    mov ecx, 512                 ; There are 512 entries in the PT.

    add_page_entry:
        mov dword[edi], ebx      ; Set PT entry to the address in EBX.
        add ebx, 0x1000          ; Move to the next 4KB block.
        add edi, 8               ; Move to the next entry in PT.
        loop add_page_entry

    ; Set up Physical Address Extension (PAE) paging.
    ; PAE is required for long mode but not enabled yet.
    mov eax, cr4
    or eax, 1 << 5               ; Set the PAE-bit (5th bit) in CR4.
    mov cr4, eax

    ; Restore the state of all general-purpose registers and return.
    popad
    ret
