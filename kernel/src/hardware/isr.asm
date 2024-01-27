[extern int_handler]
[extern irq_handler]

; Macro for handling interrupts without an error code
%macro int_handler_no_err 1
global isr_%1 
isr_%1:      
    cli                     ; Disable interrupts to prevent nested interrupt
    push dword 0            ; Push a dummy error code for consistency in stack layout
    push dword %1           ; Push interrupt number to the stack
    jmp common_interrupt_handler  ; Jump to the common interrupt handler
%endmacro

; Macro for handling interrupts with an error code
%macro int_handler_err 1
global isr_%1
isr_%1:
    cli                     ; Disable interrupts to prevent nested interrupt
    push dword %1           ; Push interrupt number to the stack
    jmp common_interrupt_handler  ; Jump to the common interrupt handler
%endmacro

; Macro for handling IRQs
%macro irq_handler_m 3
global irq_%1
irq_%1:
    cli                     ; Disable interrupts to prevent nested interrupt
    push dword %2           ; Push interrupt number to the stack
    push dword %3           ; Push interrupt number to the stack
    jmp irq_common_stub     ; Jump to the common interrupt handler
%endmacro

; Macro to push all general-purpose registers onto the stack
%macro push_all_registers 0
    push rax
    push rbx
    push rcx    
    push rdx
    push rsp
    push rbp
    push rsi
    push rdi
%endmacro

; Macro to pop all general-purpose registers from the stack
%macro pop_all_registers 0
    pop rdi
    pop rsi
    pop rbp
    pop rsp
    pop rdx
    pop rcx
    pop rbx
    pop rax
%endmacro

; Common interrupt handler entry point
common_interrupt_handler:
    push_all_registers

    ; Save CPU State
    mov ax, ds
    push rax

    ; Set the segdefs to kernel segment descriptor
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax

    ; Call the isr handler
    call int_handler

    pop rax
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    pop_all_registers

    add rsp, 8          ; Removes the pushed error code and ISR number
    sti
    iretq                 ; Return from interrupt, popping IP, CS, and EFLAGS

irq_common_stub:
    push_all_registers

    ; Save CPU State
    mov ax, ds
    push rax

    ; Set the segdefs to kernel segment descriptor
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax

    ; Call the is1 handler
    call irq_handler

    pop rbx
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    pop_all_registers

    add rsp, 8          ; Removes the pushed error code and ISR number
    sti
    iretq                 ; Return from interrupt, popping IP, CS, and EFLAGS

; Interrupt handlers
int_handler_no_err 0
int_handler_no_err 1
int_handler_no_err 2
int_handler_no_err 3
int_handler_no_err 4
int_handler_no_err 5
int_handler_no_err 6
int_handler_no_err 7
int_handler_err   8
int_handler_no_err 9
int_handler_err   10
int_handler_err   11
int_handler_err   12
int_handler_err   13
int_handler_err   14
int_handler_no_err 15
int_handler_no_err 16
int_handler_no_err 17
int_handler_no_err 18
int_handler_no_err 19
int_handler_no_err 20
int_handler_no_err 21
int_handler_no_err 22
int_handler_no_err 23
int_handler_no_err 24
int_handler_no_err 25
int_handler_no_err 26
int_handler_no_err 27
int_handler_no_err 28
int_handler_no_err 29
int_handler_no_err 30
int_handler_no_err 31

; IRQs
irq_handler_m 0, 0, 32
irq_handler_m 1, 1, 33
irq_handler_m 2, 2, 34
irq_handler_m 3, 3, 35
irq_handler_m 4, 4, 36
irq_handler_m 5, 5, 37
irq_handler_m 6, 6, 38
irq_handler_m 7, 7, 39
irq_handler_m 8, 8, 40
irq_handler_m 9, 9, 41
irq_handler_m 10, 10, 42
irq_handler_m 11, 11, 43
irq_handler_m 12, 12, 44
irq_handler_m 13, 13, 45
irq_handler_m 14, 14, 46
irq_handler_m 15, 15, 47
