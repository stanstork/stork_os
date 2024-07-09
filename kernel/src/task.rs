use core::alloc::Layout;

use crate::{print, println};

#[repr(C)]
struct Registers {
    r15: u64,
    r14: u64,
    r13: u64,
    r12: u64,
    r11: u64,
    r10: u64,
    r9: u64,
    r8: u64,
    rsi: u64,
    rdi: u64,
    rdx: u64,
    rcx: u64,
    rbx: u64,
    rax: u64,
    rbp: u64,
    rip: u64,
    cs: u64,
    rflags: u64,
    rsp: u64,
    ss: u64,
}

#[repr(C)]
pub struct Task {
    pub stack_pointer: u64,
    pub stack: *mut u8,
}

fn create_task_stack(stack_size: usize, entry_point: u64, is_kernel: bool) -> *mut u8 {
    let stack = allocate_stack_memory(stack_size);
    let mut stack_top = unsafe { stack.add(stack_size) as *mut Registers };

    unsafe {
        stack_top = stack_top.offset(-1);
        *stack_top = Registers {
            r15: 0,
            r14: 0,
            r13: 0,
            r12: 0,
            r11: 0,
            r10: 0,
            r9: 0,
            r8: 0,
            rsi: 0,
            rdi: 0,
            rdx: 0,
            rcx: 0,
            rbx: 0,
            rax: 0,
            rbp: 0,
            rip: entry_point,
            cs: if is_kernel { 0x08 } else { 0x1B },
            rflags: 0x202,
            rsp: stack_top as u64 + core::mem::size_of::<Registers>() as u64,
            ss: if is_kernel { 0x10 } else { 0x23 },
        };

        // Ensure stack alignment
        stack_top = (stack_top as usize & !0xF) as *mut Registers;

        // Print the stack for debugging
        print_stack(stack, stack_size);
    }

    stack_top as *mut u8
}

fn create_task_stack2(stack: *mut u8, entry_point: u64, is_kernel: bool) -> *mut u8 {
    // let stack = allocate_stack_memory(stack_size);
    let stack_size = 4096;
    let mut stack_top = unsafe { stack.add(stack_size) as *mut Registers };

    unsafe {
        stack_top = stack_top.offset(-1);
        *stack_top = Registers {
            r15: 0,
            r14: 0,
            r13: 0,
            r12: 0,
            r11: 0,
            r10: 0,
            r9: 0,
            r8: 0,
            rsi: 0,
            rdi: 0,
            rdx: 0,
            rcx: 0,
            rbx: 0,
            rax: 0,
            rbp: 0,
            rip: entry_point,
            cs: if is_kernel { 0x08 } else { 0x1B },
            rflags: 0x202,
            rsp: stack_top as u64 + core::mem::size_of::<Registers>() as u64,
            ss: if is_kernel { 0x10 } else { 0x23 },
        };

        // Ensure stack alignment
        stack_top = (stack_top as usize & !0xF) as *mut Registers;

        // Print the stack for debugging
        print_stack(stack, stack_size);
    }

    stack_top as *mut u8
}

pub fn allocate_stack_memory(stack_size: usize) -> *mut u8 {
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
    let stack = create_task_stack(stack_size, entry_point, true);

    Task {
        stack_pointer: stack as u64,
        stack,
    }
}

pub fn create_user_task(entry_point: u64, stack: *mut u8) -> Task {
    // let stack_size = 4096; // Adjust the stack size as needed
    let stack = create_task_stack2(stack, entry_point, false);

    Task {
        stack_pointer: stack as u64,
        stack,
    }
}

fn print_stack(stack: *mut u8, stack_size: usize) {
    unsafe {
        let stack_top = stack.add(stack_size) as *const u64;

        println!("Stack content from top to bottom:");
        for i in (0..stack_size / 8).rev() {
            let value = *stack_top.offset(-(i as isize));
            print!("0x{:x} ", value);
        }
        println!();
    }
}
