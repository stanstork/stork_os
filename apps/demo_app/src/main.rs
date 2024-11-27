#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[no_mangle]
pub extern "C" fn syscall(number: usize, arg1: usize, arg2: usize, arg3: usize) -> usize {
    let result: usize;
    unsafe {
        core::arch::asm!(
            "int 0x80",
            in("rax") number,     // Syscall number
            in("rdi") arg1,       // First argument
            in("rsi") arg2,       // Second argument
            in("r10") arg3,       // Third argument
            lateout("rax") result, // Return value in rax
            options(nostack)
        );
    }
    result
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    unsafe {
        let greet = "Hello from Rust Demo User App!\n";
        syscall(2, greet.as_ptr() as usize, greet.len(), 0);
    }

    loop {}
}
