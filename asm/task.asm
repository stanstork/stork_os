[bits 64]

global context_switch

context_switch:
    ;Save current task state
    mov [rdi], rsp
    ;Load new task state
    mov rsp, [rsi]
    ;Return to new task
    ret