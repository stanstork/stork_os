#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points

use core::panic::PanicInfo;

#[no_mangle] // don't mangle the name of this function
pub extern "C" fn _start() -> ! {
    // Create a raw pointer to a volatile unsigned char, pointing to the first text cell of video memory
    let video_memory = 0xb8000 as *mut u8;

    unsafe {
        // At the address pointed to by video_memory, store the character 'Y' and its attribute byte
        *video_memory = b'Y';
        *video_memory.add(1) = 0x07; // Attribute byte (light grey on black background)
    }

    loop {}
}

// this function is called on panic
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
