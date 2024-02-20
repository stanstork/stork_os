#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points
#![feature(abi_x86_interrupt)] // enable x86 interrupts

use core::{arch::asm, panic::PanicInfo};

use drivers::screen::display::Display;
use structures::BootInfo;

mod cpu;
mod drivers;
mod gdt;
mod interrupts;
mod structures;

#[no_mangle] // don't mangle the name of this function
pub extern "C" fn _start(boot_info: &'static BootInfo) -> ! {
    Display::init_display(&boot_info.framebuffer, &boot_info.font);

    cls!(); // clear the screen
    println!("Welcome to the StorkOS!"); // print a welcome message

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
