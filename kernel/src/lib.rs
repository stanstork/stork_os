#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points
#![feature(abi_x86_interrupt)] // enable x86 interrupts

use crate::{gdt::gdt_init, interrupts::isr::isr_install};
use core::{arch::asm, panic::PanicInfo};

mod cpu;
mod drivers;
mod gdt;
mod interrupts;
mod structures;

#[no_mangle] // don't mangle the name of this function
pub extern "C" fn _start() -> ! {
    disable_interrupts(); // Disable interrupts

    cls!();
    println!("Welcome to the kernel!");

    gdt_init(); // Initialize the Global Descriptor Table (GDT)
    isr_install(); // Initialize the Interrupt Descriptor Table (IDT) and the Programmable Interrupt Controller (PIC)

    enable_interrupts(); // Enable interrupts

    loop {}
}

// this function is called on panic
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

fn disable_interrupts() {
    unsafe {
        asm!("cli");
    }
}

fn enable_interrupts() {
    unsafe {
        asm!("sti");
    }
}
