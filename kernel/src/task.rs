use core::alloc::Layout;

#[repr(C)]
pub struct Task {
    pub stack_pointer: u64,
    stack: *mut u8,
}

fn create_kernel_task_stack(stack_size: usize, entry_point: u64) -> *mut u8 {
    let stack = allocate_stack_memory(stack_size);
    let mut stack_top = unsafe { stack.add(stack_size) as *mut u64 };

    unsafe {
        // Push initial values for general-purpose registers
        stack_top = stack_top.offset(-1);
        *stack_top = 0; // r15
        stack_top = stack_top.offset(-1);
        *stack_top = 0; // r14
        stack_top = stack_top.offset(-1);
        *stack_top = 0; // r13
        stack_top = stack_top.offset(-1);
        *stack_top = 0; // r12
        stack_top = stack_top.offset(-1);
        *stack_top = 0; // r11
        stack_top = stack_top.offset(-1);
        *stack_top = 0; // r10
        stack_top = stack_top.offset(-1);
        *stack_top = 0; // r9
        stack_top = stack_top.offset(-1);
        *stack_top = 0; // r8
        stack_top = stack_top.offset(-1);
        *stack_top = 0; // rdi
        stack_top = stack_top.offset(-1);
        *stack_top = 0; // rsi
        stack_top = stack_top.offset(-1);
        *stack_top = 0; // rdx
        stack_top = stack_top.offset(-1);
        *stack_top = 0; // rcx
        stack_top = stack_top.offset(-1);
        *stack_top = 0; // rbx
        stack_top = stack_top.offset(-1);
        *stack_top = 0; // rax
        stack_top = stack_top.offset(-1);
        *stack_top = 0; // rbp

        stack_top = stack_top.offset(-1);
        *stack_top = 0x10; // ss
        stack_top = stack_top.offset(-1);
        *stack_top = stack_top as u64 + 8 * 17; // rsp
        stack_top = stack_top.offset(-1);
        *stack_top = 0x202; // rflags
        stack_top = stack_top.offset(-1);
        *stack_top = 0x08; // cs
        stack_top = stack_top.offset(-1);
        *stack_top = entry_point; // rip
    }

    stack_top as *mut u8
}

fn allocate_stack_memory(stack_size: usize) -> *mut u8 {
    unsafe { alloc::alloc::alloc_zeroed(Layout::from_size_align(stack_size, 16).unwrap()) }
}

extern "C" {
    fn context_switch(old_stack: *mut u64, new_stack: *const u64);
}

pub unsafe fn switch_task(old_task: &mut Task, new_task: &Task) {
    context_switch(&mut old_task.stack_pointer, &new_task.stack_pointer);
}

pub fn create_kernel_task(entry_point: u64) -> Task {
    let stack_size = 4096; // Adjust the stack size as needed
    let stack = create_user_task_stack(stack_size, entry_point);

    Task {
        stack_pointer: stack as u64,
        stack,
    }
}

fn create_user_task_stack(stack_size: usize, entry_point: u64) -> *mut u8 {
    let stack = allocate_stack_memory(stack_size);
    let mut stack_top = unsafe { stack.add(stack_size) as *mut u64 };

    unsafe {
        // Push initial values for general-purpose registers
        stack_top = stack_top.offset(-1);
        *stack_top = 0; // r15
        stack_top = stack_top.offset(-1);
        *stack_top = 0; // r14
        stack_top = stack_top.offset(-1);
        *stack_top = 0; // r13
        stack_top = stack_top.offset(-1);
        *stack_top = 0; // r12
        stack_top = stack_top.offset(-1);
        *stack_top = 0; // r11
        stack_top = stack_top.offset(-1);
        *stack_top = 0; // r10
        stack_top = stack_top.offset(-1);
        *stack_top = 0; // r9
        stack_top = stack_top.offset(-1);
        *stack_top = 0; // r8
        stack_top = stack_top.offset(-1);
        *stack_top = 0; // rdi
        stack_top = stack_top.offset(-1);
        *stack_top = 0; // rsi
        stack_top = stack_top.offset(-1);
        *stack_top = 0; // rdx
        stack_top = stack_top.offset(-1);
        *stack_top = 0; // rcx
        stack_top = stack_top.offset(-1);
        *stack_top = 0; // rbx
        stack_top = stack_top.offset(-1);
        *stack_top = 0; // rax
        stack_top = stack_top.offset(-1);
        *stack_top = 0; // rbp

        // Push segment selectors and flags
        stack_top = stack_top.offset(-1);
        *stack_top = 0x23; // ss
        stack_top = stack_top.offset(-1);
        *stack_top = stack_top as u64 + 8 * 17; // rsp
        stack_top = stack_top.offset(-1);
        *stack_top = 0x202; // rflags
        stack_top = stack_top.offset(-1);
        *stack_top = 0x1B; // cs
        stack_top = stack_top.offset(-1);
        *stack_top = entry_point; // rip
    }

    stack_top as *mut u8
}

pub fn create_user_task(entry_point: u64) -> Task {
    let stack_size = 4096;
    let stack = create_kernel_task_stack(stack_size, entry_point);

    Task {
        stack_pointer: stack as u64,
        stack,
    }
}
