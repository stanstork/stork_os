#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points

use core::panic::PanicInfo;
use drivers::vga::Color;

mod drivers;
mod hardware;

#[no_mangle] // don't mangle the name of this function
pub extern "C" fn _start() -> ! {
    let mut vga = drivers::vga::VgaWriter::new();

    vga.clear_screen();
    vga.set_cursor_pos(0, 0);

    let message = "Hello World! ";
    vga.write(message, Color::Blk, Color::Cyn);

    let message = "This is a test of the VGA driver. It should print two lines. ";
    vga.write(message, Color::Blk, Color::Cyn);

    let message = "This is a second line.";
    vga.write(message, Color::Blk, Color::Cyn);

    loop {}
}

// this function is called on panic
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
