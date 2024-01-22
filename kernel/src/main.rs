#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points

use core::{panic::PanicInfo, str};

const VGA_START: usize = 0xB8000;
const VGA_EXTENT: usize = 80 * 25;
const WHITE_ON_BLACK: u8 = 0x0F;

#[repr(C)]
#[derive(Clone, Copy)]
struct VgaChar {
    ascii_char: u8,
    color_code: u8,
}

static mut VGA_BUFFER: *mut VgaChar = VGA_START as *mut VgaChar;

fn clear_screen() {
    let blank = VgaChar {
        ascii_char: b' ',
        color_code: WHITE_ON_BLACK,
    };

    for i in 0..VGA_EXTENT {
        unsafe {
            *VGA_BUFFER.add(i) = blank;
        }
    }
}

fn print(str: &str) {
    let mut i = 0;
    for byte in str.bytes() {
        if i >= VGA_EXTENT {
            break;
        }

        unsafe {
            *VGA_BUFFER.add(i) = VgaChar {
                ascii_char: byte,
                color_code: WHITE_ON_BLACK,
            };
        }
        i += 1;
    }
}

#[no_mangle] // don't mangle the name of this function
pub extern "C" fn _start() -> ! {
    clear_screen();
    print("Hello World from Rust kernel!");

    loop {}
}

// this function is called on panic
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
