use core::arch::asm;

/// Switches the current task to a new task by saving the state of the current task
/// and restoring the state of the new task. This function disables interrupts,
/// saves all general-purpose registers, updates the stack pointers, restores the
/// registers for the new task, re-enables interrupts, and returns from the interrupt,
/// effectively switching execution to the new task.    
#[naked]
pub extern "C" fn switch(old_stack: &mut u64, new_stack: &u64) {
    unsafe {
        asm!(
            // Disable interrupts to prevent race conditions during the context switch
            "cli",
            // Save all general-purpose registers
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
            // Save the current stack pointer into the old_stack variable
            "mov [rdi], rsp",
            // Load the new stack pointer from the new_stack variable
            "mov rsp, [rsi]",
            // Set task switched flag
            "mov rax, cr0",
            "or rax, 8",
            "mov cr0, rax",
            // Restore all general-purpose registers
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
            // Re-enable interrupts
            "sti",
            // Return from interrupt, effectively switching to the new task
            "iretq",
            options(noreturn)
        );
    }
}
