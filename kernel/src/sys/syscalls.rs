use crate::{interrupts::handlers::isr::InterruptStackFrame, print, println, sys::SYS_WRITE};
use alloc::string::String;
use core::arch::asm;

#[no_mangle]
pub extern "C" fn sys_write(_fd: usize, buffer: *mut u8, len: usize) -> isize {
    let string = unsafe { String::from_raw_parts(buffer, len, len) };
    print!("{}", string);
    core::mem::forget(string);
    len as isize
}

/// Handle system calls
pub extern "x86-interrupt" fn handle_system_call(_frame: &mut InterruptStackFrame) {
    let syscall_number: u64; // rax
    let arg1: u64; // rdi
    let arg2: u64; // rsi
    let arg3: u64; // r10

    // Get the syscall number and arguments
    unsafe {
        asm!(
            "mov {0}, rax",   // Syscall number
            "mov {1}, rdi",   // First argument
            "mov {2}, rsi",   // Second argument
            "mov {3}, r10",   // Third argument
            out(reg) syscall_number,
            out(reg) arg1,
            out(reg) arg2,
            out(reg) arg3,
            options(nostack)
        );
    }

    println!(
        "System call: number={}, arg1={}, arg2={}, arg3={}",
        syscall_number, arg1, arg2, arg3
    );

    match syscall_number as usize {
        1 => {
            println!("Exit syscall");
            loop {}
        }
        SYS_WRITE => {
            let buffer = arg1 as *mut u8;
            let len = arg2 as usize;
            let res = sys_write(1, buffer, len);
            unsafe {
                asm!("mov rax, {}", in(reg) res, options(nostack));
            }
        }
        _ => {
            println!("Unknown syscall: {}", syscall_number);
        }
    }
}
