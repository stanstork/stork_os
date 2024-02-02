#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points
#![feature(abi_x86_interrupt)] // enable x86 interrupts

use core::{arch::asm, panic::PanicInfo};

use interrupts::isr::isr_install;

mod drivers;
mod interrupts;

#[no_mangle] // don't mangle the name of this function
pub extern "C" fn _start() -> ! {
    cls!();
    isr_install();

    // Non-maskable interrupt
    unsafe {
        asm!("int 0x2");
    }

    // Breakpoint
    unsafe {
        asm!("int 0x3");
    }

    loop {}
}

// this function is called on panic
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
