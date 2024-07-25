use core::arch::asm;

macro_rules! save_context {
    () => {
        concat!(
            r#"
            cli
			push rbp
            push rdi
            push rsi
            push rdx
            push rcx
            push rbx
            push rax
            push r8
            push r9
            push r10
            push r11
            push r12
            push r13
            push r14
            push r15
			"#,
        )
    };
}

macro_rules! restore_context {
    () => {
        concat!(
            r#"
			pop r15
			pop r14
			pop r13
			pop r12
			pop r11
			pop r10
			pop r9
			pop r8
            pop rax
            pop rbx
            pop rcx
            pop rdx
            pop rsi
            pop rdi
            pop rbp
            sti
			iretq
			"#
        )
    };
}

#[naked]
pub extern "C" fn switch_to_task(old_stack: &mut u64, new_stack: &u64) {
    unsafe {
        asm!(
            // disable interrupts
            "cli",
            // save registers
            "push r15",
            "push r14",
            "push r13",
            "push r12",
            "push r11",
            "push r10",
            "push r9",
            "push r8",
            "push rbp",
            "push rdi",
            "push rsi",
            "push rdx",
            "push rcx",
            "push rbx",
            "push rax",
            // save the current stack pointer in old_stack
            "mov [rdi], rsp",
            // load the new stack pointer from new_stack
            "mov rsp, [rsi]",
            // restore registers
            "pop rax",
            "pop rbx",
            "pop rcx",
            "pop rdx",
            "pop rsi",
            "pop rdi",
            "pop rbp",
            "pop r8",
            "pop r9",
            "pop r10",
            "pop r11",
            "pop r12",
            "pop r13",
            "pop r14",
            "pop r15",
            // re-enable interrupts
            "sti",
            // return from interrupt
            "iretq",
            options(noreturn)
        );
    }
}
