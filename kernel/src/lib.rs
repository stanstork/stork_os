#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points
#![feature(abi_x86_interrupt)] // enable x86 interrupts
#![feature(asm_const)] // enable inline assembly

use core::{arch::asm, panic::PanicInfo};

use drivers::vga::{Color, ColorCode};
use hardware::{idt::init_gdt, isr::isr_install};

mod drivers;
mod hardware;

#[no_mangle] // don't mangle the name of this function
pub extern "C" fn _start() -> ! {
    //init_gdt();
    isr_install();

    // let mut vga = drivers::vga::VgaWriter::new();

    // vga.clear_screen();
    // vga.move_cursor(0, 24);

    // let color_code = ColorCode::new(Color::Wht, Color::Dgy);

    // vga.write("Hello, world!\n", color_code);
    // vga.write("New line\n", color_code);
    // vga.write("New line 2", color_code);

    // let num = 42;
    // vga.write("\nThe answer is ", color_code);
    // vga.write_num(num, color_code);

    //invoke a breakpoint exception
    unsafe {
        asm!("int 0x3");
    }

    unsafe {
        asm!("int 0x4");
    }

    loop {}
}

// this function is called on panic
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
