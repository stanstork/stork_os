#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points

use core::panic::PanicInfo;
use drivers::vga::{Color, ColorCode};

mod drivers;
mod hardware;

#[no_mangle] // don't mangle the name of this function
pub extern "C" fn _start() -> ! {
    let mut vga = drivers::vga::VgaWriter::new();

    vga.clear_screen();
    vga.move_cursor(0, 24);

    let color_code = ColorCode::new(Color::Wht, Color::Dgy);

    vga.write("Hello, world!\n", color_code);
    vga.write("New line\n", color_code);
    vga.write("New line 2", color_code);

    loop {}
}

// this function is called on panic
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
