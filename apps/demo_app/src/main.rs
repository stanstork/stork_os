#![no_std]
#![no_main]

extern crate std;

use core::panic::PanicInfo;
use std::print;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    print!("Hello, {}!\n", "world");
    print!("The answer is: {}\n", 42);

    loop {}
}
