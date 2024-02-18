#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points
#![feature(abi_x86_interrupt)] // enable x86 interrupts

use core::{arch::asm, panic::PanicInfo};

use structures::BootInfo;

mod cpu;
mod drivers;
mod gdt;
mod interrupts;
mod structures;

#[no_mangle] // don't mangle the name of this function
pub extern "C" fn _start(boot_info: &BootInfo) -> ! {
    // Test uefi boot

    let color = 0x00FF_00FF; // purple
    let address = boot_info.framebuffer.pointer as *mut u32;

    memset(
        address,
        color,
        (boot_info.framebuffer.width * boot_info.framebuffer.height) as usize,
    ); // fill the screen with purple

    loop {} // return an exit code
}

fn memset(dest: *mut u32, val: u32, count: usize) {
    unsafe {
        for i in 0..count {
            *dest.add(i) = val;
        }
    }
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
