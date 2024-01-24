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
    vga.cursor.set_position(0, 24);

    vga.write("Hello, world!\n", ColorCode::new(Color::Blk, Color::Cyn));
    vga.write("New line\n", ColorCode::new(Color::Blk, Color::Cyn));
    vga.write("New line 2", ColorCode::new(Color::Blk, Color::Cyn));

    loop {}
}

// this function is called on panic
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
